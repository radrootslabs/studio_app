import { WebAesGcmCipher, type WebAesGcmCipherConfig } from "@radroots/client/cipher";
import { cfg_data } from "../config";

const sql_cipher_config = (store_key: string): WebAesGcmCipherConfig => ({
    idb_config: cfg_data.sql_cipher,
    key_name: `radroots.sql.${store_key}.aes-gcm.key`
});

export const reset_sql_cipher = async (store_key: string): Promise<void> => {
    const cipher = new WebAesGcmCipher(sql_cipher_config(store_key));
    const res = await cipher.reset();
    if (res && "err" in res) throw new Error(res.err);
};
