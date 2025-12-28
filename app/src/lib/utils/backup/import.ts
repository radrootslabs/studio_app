import { datastore, db, nostr_keys } from "$lib/utils/app";
import { ls } from "$lib/utils/i18n";
import { get_store, handle_err, parse_file_json } from "@radroots/apps-lib";
import type { ImportableAppState } from "@radroots/apps-lib-pwa/types/app";
import type { IError } from "@radroots/types-bindings";
import type { IdbClientConfig } from "@radroots/utils";
import { err_msg, throw_err } from "@radroots/utils";
import { createStore, set as idb_set } from "idb-keyval";
import { reset_sql_cipher } from "../app/cipher";
import { app_cfg } from "../app/config";

const ls_val = get_store(ls);

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
            ls_val(`error.configuration.import.storage_mismatch`, {
                label,
                app_database: current.database,
                app_store: current.store,
                backup_database: incoming.database,
                backup_store: incoming.store,
            })
        );
    }
};

export const validate_import_file = async (file: File | null): Promise<ImportableAppState> => {
    const parsed_res = await parse_file_json(file);
    if (!parsed_res.ok) throw_err(ls_val(`error.configuration.import.invalid_file_contents`));
    return await validate_import_state(parsed_res.value);
};

export const validate_import_state = async (state: any): Promise<ImportableAppState> => {
    if (!state || typeof state !== "object") throw_err(ls_val(`error.configuration.import.empty_file`));
    if (state.backup_version !== app_cfg.backup.version) {
        throw_err(
            ls_val(`error.configuration.import.unsupported_backup_version`, {
                backup_version: state.backup_version,
                expected_version: app_cfg.backup.version,
            })
        );
    }
    const backup_format = state?.versions?.backup_format ?? state?.versions?.dump_format;
    if (!state.versions || !state.versions.app || !state.versions.tangle_db || !backup_format) {
        throw_err(ls_val(`error.configuration.import.missing_version_metadata`));
    }
    const database = state.database ?? state.tangle_db;
    const backup = database?.backup ?? database?.dump;
    if (!state.datastore || !state.nostr_keystore || !database) {
        throw_err(ls_val(`error.configuration.import.missing_required_sections`));
    }
    if (!backup || backup.format_version !== backup_format) {
        throw_err(ls_val(`error.configuration.import.database_format_mismatch`));
    }
    if (backup.tangle_db_version !== state.versions.tangle_db) {
        throw_err(ls_val(`error.configuration.import.tangle_db_version_mismatch`));
    }
    return {
        ...state,
        versions: { ...state.versions, backup_format },
        database: { ...database, backup }
    };
};


const restore_datastore_state = async (state: ImportableAppState["datastore"]): Promise<void> => {
    const ds_cfg = datastore.get_config();
    assert_config_match(ds_cfg, state.config, ls_val(`common.datastore`));
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
    assert_config_match(nostr_cfg, state.config, ls_val(`common.nostr_keystore`));
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
            ls_val(`error.configuration.import.tangle_store_key_mismatch`, {
                app_store_key: current_store_key,
                backup_store_key: state.store_key,
            })
        );
    }
    await reset_sql_cipher(current_store_key);
    await db.reinit();
    await db.import_backup(state.backup);
};

export const import_app_state = async (payload: ImportableAppState): Promise<AppImportStateResult | IError<string>> => {
    try {
        if (typeof window === "undefined") return err_msg(ls_val(`error.client.undefined_window`));
        const import_state = await validate_import_state(payload);
        assert_config_match(datastore.get_config(), import_state.datastore.config, ls_val(`common.datastore`));
        assert_config_match(nostr_keys.get_config(), import_state.nostr_keystore.config, ls_val(`common.nostr_keystore`));
        const current_store_key = db.get_store_key();
        if (current_store_key !== import_state.database.store_key) {
            const message = ls_val(`error.configuration.import.tangle_store_key_mismatch`, {
                app_store_key: current_store_key,
                backup_store_key: import_state.database.store_key,
            });
            console.error(message);
            return err_msg(message);
        }
        await restore_datastore_state(import_state.datastore);
        await restore_nostr_keystore_state(import_state.nostr_keystore);
        await restore_tangle_db_state(import_state.database);

        return { pass: true }
    } catch (e) {
        handle_err(e, `import_app_state`);
        return { pass: false, message: ls_val(`error.configuration.import.failure`) }
    }
};

export const import_app_state_from_file = async (file: File): Promise<ReturnType<typeof import_app_state>> => {
    const validated = await validate_import_file(file);
    return await import_app_state(validated);
};
