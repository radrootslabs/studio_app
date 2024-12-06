import { PUBLIC_RADROOTS_URL } from "$env/static/public";
import { http, nostr } from "$lib/client";
import { cfg } from "$lib/conf";
import { get_store, t } from "@radroots/svelte-lib";
import { err_msg, type ErrorMessage } from "@radroots/utils";
import { catch_err, throw_err } from "./error";

const $t = get_store(t);

export const fetch_radroots_profile_validate = async (opts: {
    profile_name: string;
}): Promise<{ profile_name: string } | ErrorMessage<string>> => {
    try {
        const res = await http.fetch({
            url: `${PUBLIC_RADROOTS_URL}/public/accounts/list`,
            method: `post`,
        });
        console.log(JSON.stringify(res, null, 4), `res`);
        if (`err` in res) throw_err(res)
        else if (res.error) throw_err(res.error.message)
        else if (res.data && Array.isArray(res.data.results)) {
            const existing_profile = res.data.results.find(
                (i: any) =>
                    `nip_05` in i &&
                    String(i.nip_05).toLowerCase() ===
                    opts.profile_name.toLowerCase(),
            );
            if (existing_profile)
                return err_msg(
                    `${`${$t(`icu.the_*_is_registered`, { value: `${$t(`common.profile_name`)}`.toLowerCase() })} `}`,
                );
            return { profile_name: opts.profile_name };
        }
        return err_msg(`${$t(`error.client.request_failure`)}`);
    } catch (e) {
        await catch_err(e, "fetch_radroots_profile_validate");
        return err_msg(`${$t(`error.client.network_failure`)}`);
    }
};

export const fetch_radroots_profile_init = async (opts: {
    profile_name: string;
    secret_key: string;
    nostr_relays?: string[];
}): Promise<{ tok: string } | ErrorMessage<string>> => {
    try {
        const res = await http.fetch({
            url: `${PUBLIC_RADROOTS_URL}/public/accounts/add/init`,
            method: `post`,
            data: {
                nip_05: opts.profile_name,
                public_key: nostr.lib.public_key(opts.secret_key),
                nostr_relays: opts.nostr_relays?.length
                    ? Array.from(
                        new Set([
                            ...opts.nostr_relays,
                            cfg.nostr.relay_url,
                        ]),
                    ).join(`,`)
                    : [cfg.nostr.relay_url].join(`,`),
            },
        });
        if (`err` in res) return res;
        else if (res.data && `tok` in res.data) {
            return { tok: res.data.tok };
        } else if (res.data && `message` in res.data)
            return err_msg(
                `${$t(`radroots-org.error.${res.data.message}`)}`,
            );
        return err_msg(`${$t(`error.client.request_failure`)}`);
    } catch (e) {
        await catch_err(e, "fetch_radroots_profile_init");
        return err_msg(`${$t(`error.client.network_failure`)}`);
    }
};

export const fetch_radroots_profile_confirm = async (
    authorization: string,
): Promise<{ pass: true } | ErrorMessage<string>> => {
    try {
        const res = await http.fetch({
            url: `${PUBLIC_RADROOTS_URL}/public/accounts/add/conf`,
            method: `post`,
            authorization,
        });
        if (`err` in res) return res;
        return { pass: true };
    } catch (e) {
        await catch_err(e, "fetch_radroots_profile_confirm");
        return err_msg(`${$t(`error.client.network_failure`)}`);
    }
};

export const fetch_radroots_profile_status = async (
    authorization: string,
): Promise<
    | { active: { public_key: string; nip_05?: string } }
    | ErrorMessage<string>
> => {
    try {
        const res = await http.fetch({
            url: `${PUBLIC_RADROOTS_URL}/public/accounts/add/status`,
            method: `post`,
            authorization,
        });
        if (`err` in res) return res;
        else if (
            `public_key` in res.data &&
            typeof res.data.public_key === `string`
        )
            return {
                active: {
                    public_key: res.data.public_key,
                    nip_05:
                        `nip_05` in res.data &&
                            typeof res.data.nip_05 === `string`
                            ? res.data.nip_05
                            : undefined,
                },
            };
        return err_msg(`${$t(`error.client.network_failure`)}`);
    } catch (e) {
        await catch_err(e, "fetch_radroots_profile_status");
        return err_msg(`${$t(`error.client.network_failure`)}`);
    }
};