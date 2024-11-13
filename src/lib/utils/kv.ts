import { fmt_id, kv } from "@radroots/svelte-lib";

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

export const kv_sync = async (list: [string, string][]): Promise<void> => {
    try {
        for (const [key, val] of list) await kv.set(key, val);
    } catch (e) {
        console.log(`(error) kv_sync `, e);
    }
};