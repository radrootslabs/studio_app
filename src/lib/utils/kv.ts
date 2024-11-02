import { parse_trade_product_form_keys, trade_product_form_fields } from "@radroots/models";
import { fmt_id, kv } from "@radroots/svelte-lib";
import { err_msg, type ErrorMessage, type ResultPass } from "@radroots/utils";

export const kv_init_page = async (): Promise<void> => {
    try {
        const kv_pref = fmt_id();
        const range = Keyva.prefix(kv_pref);
        const kv_list = await kv.each({ range }, `keys`);
        await Promise.all(kv_list.map((k) => kv.set(k, ``)));
    } catch (e) {
        console.log(`(error) kv_init_page `, e);
    }
};

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

export const validate_trade_product_fields = async (opts: {
    kv_pref: string;
    fields: string[];
}): Promise<ResultPass | ErrorMessage<string>> => {
    try {
        for (const field of opts.fields) {
            const field_k = parse_trade_product_form_keys(field);
            if (!field_k) return err_msg(field);
            const field_id = `${opts.kv_pref}-${field_k}`;
            const field_val = await kv.get(field_id);
            console.log(`${field_k}: '${field_val}'`)
            if (!trade_product_form_fields[field_k].validation.test(field_val)) return err_msg(field_k);
        }
        return { pass: true };
    } catch (e) {
        console.log(`(error) validate_trade_product_fields `, e);
        return err_msg(String(e))
    }
};