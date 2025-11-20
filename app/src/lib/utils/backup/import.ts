import { datastore, db, nostr_keys } from "$lib/utils/app";
import { handle_err, parse_file_json } from "@radroots/apps-lib";
import type { ImportableAppState } from "@radroots/apps-lib-pwa/types/app";
import type { IError } from "@radroots/types-bindings";
import type { IdbClientConfig } from "@radroots/utils";
import { err_msg, throw_err } from "@radroots/utils";
import { createStore, set as idb_set } from "idb-keyval";
import { reset_sql_cipher } from "../app/cipher";
import { app_cfg } from "../app/config";

export type AppImportStateResult = {
    pass: boolean;
    message?: string;
}

const assert_config_match = (
    current: IdbClientConfig,
    incoming: IdbClientConfig,
    label: string
): void => {
    if (current.database !== incoming.database || current.store !== incoming.store) {
        throw_err(
            `Import failed: ${label} storage mismatch (app=${current.database}/${current.store}, backup=${incoming.database}/${incoming.store})`
        );
    }
};

export const validate_import_file = async (file: File | null): Promise<ImportableAppState> => {
    const parsed: any = await parse_file_json(file)
    if (!parsed) throw_err("error.app.import.invalid_file_contents")
    return await validate_import_state(parsed);
};

export const validate_import_state = async (state: any): Promise<ImportableAppState> => {
    if (!state || typeof state !== "object") throw_err("Import file is empty.");
    if (state.backup_version !== app_cfg.backup.version) {
        throw_err(
            `Unsupported backup version (${state.backup_version}); expected ${app_cfg.backup.version}.`
        );
    }
    const backup_format = state?.versions?.backup_format ?? state?.versions?.dump_format;
    if (!state.versions || !state.versions.app || !state.versions.tangle_sql || !backup_format) {
        throw_err("Import file is missing version metadata.");
    }
    const database = state.database ?? state.tangle_db;
    const backup = database?.backup ?? database?.dump;
    if (!state.datastore || !state.nostr_keystore || !database) {
        throw_err("Import file is missing required sections.");
    }
    if (!backup || backup.format_version !== backup_format) {
        throw_err("Import file database backup format does not match metadata.");
    }
    if (backup.tangle_sql_version !== state.versions.tangle_sql) {
        throw_err("Import file tangle-sql version does not match metadata.");
    }
    return {
        ...state,
        versions: { ...state.versions, backup_format },
        database: { ...database, backup }
    };
};


const restore_datastore_state = async (state: ImportableAppState["datastore"]): Promise<void> => {
    const ds_cfg = datastore.get_config();
    assert_config_match(ds_cfg, state.config, "datastore");
    const reset_res = await datastore.reset();
    if (reset_res && "err" in reset_res) throw_err(reset_res.err);
    const store = createStore(ds_cfg.database, ds_cfg.store);
    const entries = Object.entries(state.entries);
    for (const [key, value] of entries) {
        await idb_set(key, value, store);
    }
};

const restore_nostr_keystore_state = async (
    state: ImportableAppState["nostr_keystore"]
): Promise<void> => {
    const nostr_cfg = nostr_keys.get_config();
    assert_config_match(nostr_cfg, state.config, "nostr keystore");
    const reset_res = await nostr_keys.reset();
    if (reset_res && "err" in reset_res) throw_err(reset_res.err);
    for (const key of state.keys) {
        const add_res = await nostr_keys.add(key.secret_key);
        if ("err" in add_res) throw_err(add_res.err);
    }
};

const restore_tangle_db_state = async (state: ImportableAppState["database"]): Promise<void> => {
    const current_store_key = db.get_store_key();
    if (current_store_key !== state.store_key) {
        throw_err(
            `Import failed: tangle DB store key mismatch (app=${current_store_key}, backup=${state.store_key}).`
        );
    }
    await reset_sql_cipher(current_store_key);
    await db.reinit();
    await db.import_backup(state.backup);
};

export const import_app_state = async (payload: ImportableAppState): Promise<AppImportStateResult | IError<string>> => {
    try {
        if (typeof window === "undefined") return err_msg("error.client.undefined_window");
        const import_state = await validate_import_state(payload);
        assert_config_match(datastore.get_config(), import_state.datastore.config, "datastore");
        assert_config_match(nostr_keys.get_config(), import_state.nostr_keystore.config, "nostr keystore");
        const current_store_key = db.get_store_key();
        if (current_store_key !== import_state.database.store_key) {
            console.error(`Import failed: tangle DB store key mismatch (app=${current_store_key}, backup=${import_state.database.store_key}).`);
            return err_msg("error.database.store_key_mismatch");
        }
        await restore_datastore_state(import_state.datastore);
        await restore_nostr_keystore_state(import_state.nostr_keystore);
        await restore_tangle_db_state(import_state.database);

        return { pass: true }
    } catch (e) {
        handle_err(e, `import_app_state`);
        return { pass: false, message: "Unable to import app state. Please verify the file and try again." }
    }
};

export const import_app_state_from_file = async (file: File): Promise<ReturnType<typeof import_app_state>> => {
    const validated = await validate_import_file(file);
    return await import_app_state(validated);
};
