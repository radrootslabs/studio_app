import { parse_trade_product_form_keys, trade_product_form_fields, trade_product_form_vals, type TradeProductFormFields } from "@radroots/models";
import { kv } from "@radroots/svelte-lib";
import { err_msg, type ErrorMessage } from "@radroots/utils";

export const kv_init_trade_product_fields = async (kv_pref: string): Promise<void> => {
    try {
        for (const k of Object.keys(
            trade_product_form_fields,
        )) {
            const field_k = parse_trade_product_form_keys(k);
            if (!field_k) continue;
            const field_id = `${kv_pref}-${field_k}`
            await kv.delete(field_id);
        }

    } catch (e) {
        console.log(`(error) kv_init_trade_product_fields `, e);
    }
};

export const kv_validate_trade_product_fields = async (opts: {
    kv_pref: string;
    no_validation?: string[] | true;
}): Promise<TradeProductFormFields | ErrorMessage<string>> => {
    try {
        let no_validation = opts.no_validation || [];
        const vals = {
            ...trade_product_form_vals
        };

        for (const [k, field] of Object.entries(
            trade_product_form_fields,
        )) {
            const field_k = parse_trade_product_form_keys(k);
            if (!field_k) continue;
            const field_id = `${opts.kv_pref}-${field_k}`;
            const field_val = await kv.get(field_id);
            if (field_val) vals[field_k] = field_val;
            if (no_validation === true) continue;
            else if (
                (!field.optional && !field.validation.test(field_val)) ||
                (field.optional &&
                    field_val &&
                    !field.validation.test(field_val))
            ) {
                if (no_validation.includes(field_k)) continue;
                else return err_msg(field_k);
            }
        }
        return vals;
    } catch (e) {
        console.log(`(error) trade_product_submit_preview `, e);
        return err_msg(String(e))
    }
};