import { parse_trade_product_form_keys, trade_product_form_fields, trade_product_form_vals, type IModelsForm, type TradeProductFormFields } from "@radroots/models";
import { fmt_id, kv } from "@radroots/svelte-lib";
import { err_msg, type ErrorMessage } from "@radroots/utils";

const trade_products_field_validate = (field: IModelsForm, field_val: string): boolean => {
    if (
        (!field.optional && !field.validation.test(field_val)) ||
        (field.optional &&
            field_val &&
            !field.validation.test(field_val))
    ) return false;
    return true;
};

export const trade_product_fields_assign = async (opts?: {
    kv_pref?: string;
    field_defaults?: [keyof typeof trade_product_form_vals, string][];
    field_pass?: string[] | true;
}): Promise<TradeProductFormFields | ErrorMessage<string>> => {
    try {
        const fields = {
            ...trade_product_form_vals
        };
        for (const [field_ks, field] of Object.entries(
            trade_product_form_fields,
        )) {
            const field_k = parse_trade_product_form_keys(field_ks);
            if (!field_k) continue;
            const field_val = await kv.get(`${opts?.kv_pref || fmt_id()}-${field_k}`);
            if (field_val) fields[field_k] = field_val;
        }
        if (opts?.field_defaults && opts?.field_defaults?.length > 0) for (const [field_k, field_v] of opts?.field_defaults) if (!fields[field_k]) fields[field_k] = field_v;
        return fields;
    } catch (e) {
        console.log(`(error) trade_product_fields_assign `, e);
        return err_msg(String(e))
    }
};

export const trade_product_fields_validate = async (opts: {
    kv_pref?: string;
    field_defaults?: [keyof typeof trade_product_form_vals, string][];
    fields_pass?: string[];
}): Promise<TradeProductFormFields | ErrorMessage<string>> => {
    try {
        const fields = {
            ...trade_product_form_vals
        };
        for (const [field_ks, field] of Object.entries(
            trade_product_form_fields,
        )) {
            const field_k = parse_trade_product_form_keys(field_ks);
            if (!field_k) continue;
            const field_val = await kv.get(`${opts?.kv_pref || fmt_id()}-${field_k}`);
            if (field_val) fields[field_k] = field_val;
        }
        if (opts?.field_defaults && opts?.field_defaults?.length > 0) for (const [field_k, field_v] of opts?.field_defaults) if (!fields[field_k]) fields[field_k] = field_v;

        for (const [field_ks, field_val] of Object.entries(fields)) {
            const field_k = parse_trade_product_form_keys(field_ks);
            if (!field_k) continue;
            const field = trade_product_form_fields[field_k]
            if (!trade_products_field_validate(field, field_val)) {
                if (opts.fields_pass?.includes(field_k)) continue;
                return err_msg(field_k);
            }
        }
        return fields;
    } catch (e) {
        console.log(`(error) trade_product_fields_validate `, e);
        return err_msg(String(e))
    }
};

export const trade_product_fields_kv_validate = async (opts?: {
    kv_pref?: string;
    fields_pass?: string[] | true;
}): Promise<TradeProductFormFields | ErrorMessage<string>> => {
    try {
        const vals = {
            ...trade_product_form_vals
        };
        for (const [k, field] of Object.entries(
            trade_product_form_fields,
        )) {
            const field_k = parse_trade_product_form_keys(k);
            if (!field_k) continue;
            const field_id = `${opts?.kv_pref || fmt_id()}-${field_k}`;
            const field_val = await kv.get(field_id);
            if (field_val) vals[field_k] = field_val;
            if (opts?.fields_pass === true) continue;
            else if (!trade_products_field_validate(field, field_val)) {
                if (opts?.fields_pass?.includes(field_k)) continue;
                else return err_msg(field_k);
            }
        }
        return vals;
    } catch (e) {
        console.log(`(error) trade_product_fields_kv_validate `, e);
        return err_msg(String(e))
    }
};
