<script lang="ts">
    import {
        PUBLIC_NOSTR_RELAY_DEFAULTS,
        PUBLIC_RADROOTS_URL,
    } from "$env/static/public";
    import { db, dialog, http, keystore, nostr } from "$lib/client";
    import { cfg, ks } from "$lib/conf";
    import { restart } from "$lib/utils/client";
    import type { IClientUnlisten } from "@radroots/client";
    import {
        app_layout,
        app_loading,
        app_splash,
        ButtonCarouselPair,
        carousel_index,
        carousel_index_max,
        carousel_next,
        carousel_prev,
        DisplayLine,
        el_id,
        EntryLine,
        fmt_id,
        Glyph,
        InputElement,
        kv,
        LayoutView,
        route,
        sleep,
        t,
        view_effect,
    } from "@radroots/svelte-lib";
    import { err_msg, regex, type ErrorMessage } from "@radroots/utils";
    import { onDestroy, onMount } from "svelte";

    const page_param: {
        kv: {
            nostr_secretkey: string;
            nostr_profilename: string;
        };
        carousel: Record<View, { max_index: number }>;
    } = {
        kv: {
            nostr_secretkey: `kv_secretkey`,
            nostr_profilename: `kv_profilename`,
        },
        carousel: {
            cfg_init: {
                max_index: 2,
            },
            cfg_main: {
                max_index: 2,
            },
            eula: {
                max_index: 1,
            },
        },
    };

    let el_eula: HTMLDivElement;
    let el_eula_scrolled = false;

    type CfgInitKeyOption = `key_gen` | `kv_nostr_secretkey`;
    let cgf_init_key_option: CfgInitKeyOption | undefined = undefined;

    type CfgMainOptionIndex1 = `farmer` | `personal`;
    let cfg_main_opt_idx1: CfgMainOptionIndex1 | undefined = undefined;
    let cfg_main_nostr_publickey = ``;
    let cfg_main_nostr_publickey_npub = ``;
    $: cfg_main_nostr_publickey_npub = cfg_main_nostr_publickey
        ? nostr.lib.npub(cfg_main_nostr_publickey) || ``
        : ``;
    let cfg_main_profilename_valid = false;
    let cfg_main_profilename_loading = false;

    type View = `cfg_init` | `cfg_main` | `eula`;
    let initial_view: View = `cfg_init`;
    let view: View = initial_view;
    $: {
        view_effect<View>(view);
    }

    let unlisten_cfg_init_nostr_secretkey: IClientUnlisten | undefined =
        undefined;

    onMount(async () => {
        try {
            handle_view(initial_view);

            const unlisten_1 = await keystore.on_key_change(
                ks.cfg_init.nostr_secretkey,
                async (_cfg_init_nostr_secretkey) => {
                    console.log(
                        `_cfg_init_nostr_secretkey `,
                        _cfg_init_nostr_secretkey,
                    );
                    if (_cfg_init_nostr_secretkey) {
                        cfg_main_nostr_publickey =
                            nostr.lib.secretkey_to_publickey(
                                _cfg_init_nostr_secretkey,
                            ) || ``;
                    } else cfg_main_nostr_publickey = ``;
                },
            );
            if (!(`err` in unlisten_1))
                unlisten_cfg_init_nostr_secretkey = unlisten_1;
            el_eula?.addEventListener(`scroll`, () => {
                const client_h = el_eula?.clientHeight;
                const scroll_h = el_eula?.scrollHeight;
                const scroll_top = el_eula?.scrollTop;
                if (scroll_top + client_h >= scroll_h) el_eula_scrolled = true;
            });
            await lookup_ks();
        } catch (e) {
            console.log(`e mount`, e);
        } finally {
            app_splash.set(false);
        }
    });

    onDestroy(async () => {
        try {
            await reset_kv();
            if (unlisten_cfg_init_nostr_secretkey)
                unlisten_cfg_init_nostr_secretkey();
            el_eula?.removeEventListener(`scroll`, () => null);
        } catch (e) {
            console.log(`e destroy`, e);
        } finally {
        }
    });

    const reset_page = async (alert_message?: string): Promise<void> => {
        try {
            app_loading.set(true);
            handle_view(`cfg_init`);
            if (alert_message) await dialog.alert(alert_message);
            await sleep(cfg.delay.load);
        } catch (e) {
            console.log(`(error) reset `, e);
        } finally {
            app_loading.set(false);
        }
    };

    const lookup_ks = async (): Promise<void> => {
        try {
            app_splash.set(false);
            const ks_nostr_secretkey = await keystore.get(
                ks.cfg_init.nostr_secretkey,
            );
            const ks_radroots_tok = await keystore.get(
                ks.cfg_init.radroots_tok,
            );
            if (`result` in ks_nostr_secretkey) {
                app_loading.set(true);
                cfg_main_nostr_publickey;
                if (`result` in ks_radroots_tok) {
                    cfg_main_nostr_publickey = nostr.lib.public_key(
                        ks_nostr_secretkey.result,
                    );
                    const profile_status = await fetch_radroots_profile_status(
                        ks_radroots_tok.result,
                    );
                    if (`active` in profile_status) {
                        if (
                            profile_status.active.public_key !==
                            cfg_main_nostr_publickey
                        ) {
                            await keystore.remove(ks.cfg_init.nostr_secretkey);
                            await keystore.remove(ks.cfg_init.radroots_tok);
                            await keystore.remove(
                                ks.cfg_init.nostr_profilename,
                            );
                            return;
                        }
                        const confirm = await dialog.confirm({
                            message: `There is an existing configuration in progress${`nip_05` in profile_status.active ? ` for "${profile_status.active.nip_05}".` : `.`} ${`${$t(`common.do_you_want_to_continue_q`)}`}`, //@todo
                            cancel_label: `${$t(`common.reset`)}`,
                            ok_label: `${$t(`common.yes`)}`,
                        });

                        if (confirm === false) {
                            await keystore.remove(ks.cfg_init.nostr_secretkey);
                            await keystore.remove(ks.cfg_init.radroots_tok);
                            await keystore.remove(
                                ks.cfg_init.nostr_profilename,
                            );
                            return;
                        } else {
                            if (`nip_05` in profile_status.active) {
                                await kv.set(
                                    page_param.kv.nostr_profilename,
                                    profile_status.active.nip_05,
                                );
                            }
                            await kv.set(
                                page_param.kv.nostr_profilename,
                                profile_status.active.nip_05,
                            );
                            handle_view(`eula`);
                        }
                    }
                } else {
                    await keystore.remove(ks.cfg_init.nostr_secretkey);
                }
            }
        } catch (e) {
            console.log(`(error) lookup_ks `, e);
        } finally {
            app_loading.set(false);
        }
    };

    const reset_ks = async (): Promise<void> => {
        try {
            const ks_entries = await keystore.entries();
            if (`results` in ks_entries) {
                for (const [ks_key] of ks_entries.results.filter(([k]) =>
                    k.startsWith(`cfg:init:`),
                ))
                    await keystore.remove(ks_key);
            }
        } catch (e) {
            console.log(`(error) reset_ks `, e);
        }
    };

    const reset_kv = async (): Promise<void> => {
        try {
            for (const kv_key of Object.values(page_param.kv))
                await kv.delete(kv_key);
        } catch (e) {
            console.log(`(error) reset_ks `, e);
        }
    };

    const handle_view = (new_view: View): void => {
        if (new_view === `cfg_init` && view === `cfg_main`) {
            const offset = cgf_init_key_option === `key_gen` ? 1 : 0;
            carousel_index.set(
                page_param.carousel[new_view].max_index - offset,
            );
        } else {
            carousel_index.set(0);
            carousel_index_max.set(page_param.carousel[new_view].max_index);
        }
        view = new_view;
    };

    const handle_back = async (): Promise<void> => {
        try {
            switch (view) {
                case `cfg_init`:
                    {
                        switch ($carousel_index) {
                            case 1:
                                {
                                    cgf_init_key_option = undefined;
                                    await carousel_prev(view);
                                }
                                break;
                            case 2:
                                {
                                    await carousel_prev(view);
                                }
                                break;
                        }
                    }
                    break;
                case `cfg_main`: {
                    switch ($carousel_index) {
                        case 0:
                            {
                                handle_view(`cfg_init`);
                            }
                            break;
                        case 1:
                            {
                                carousel_prev(view);
                            }
                            break;
                    }
                }
            }
        } catch (e) {
            console.log(`(error) handle_back `, e);
        }
    };

    const handle_continue = async (): Promise<void> => {
        try {
            switch (view) {
                case `cfg_init`:
                    {
                        switch ($carousel_index) {
                            case 0:
                                {
                                    await carousel_next(view);
                                }
                                break;
                            case 1:
                                {
                                    if (
                                        cgf_init_key_option ===
                                        `kv_nostr_secretkey`
                                    ) {
                                        await carousel_next(view);
                                        return;
                                    }
                                    const ks_nostr_secretkey =
                                        await keystore.get(
                                            ks.cfg_init.nostr_secretkey,
                                        );
                                    if (`result` in ks_nostr_secretkey) {
                                        handle_view(`cfg_main`);
                                        return;
                                    }
                                    await keystore.set(
                                        ks.cfg_init.nostr_secretkey,
                                        nostr.lib.generate_key(),
                                    );
                                    handle_view(`cfg_main`);
                                }
                                break;
                            case 2:
                                {
                                    const kv_nostr_secretkey = await kv.get(
                                        fmt_id(page_param.kv.nostr_secretkey),
                                    );
                                    if (!kv_nostr_secretkey) {
                                        await dialog.alert(
                                            `${$t(`icu.enter_a_*`, { value: `${$t(`icu.valid_*`, { value: `${$t(`common.key`)}` })}`.toLowerCase() })}`,
                                        );
                                        return;
                                    }
                                    const nostr_secretkey_valid_nsec =
                                        nostr.lib.nsec_decode(
                                            kv_nostr_secretkey,
                                        );
                                    const nostr_secretkey_valid_hex =
                                        nostr.lib.public_key(
                                            kv_nostr_secretkey,
                                        );
                                    if (
                                        nostr_secretkey_valid_nsec ||
                                        nostr_secretkey_valid_hex
                                    ) {
                                        await keystore.set(
                                            ks.cfg_init.nostr_secretkey,
                                            kv_nostr_secretkey,
                                        );
                                        handle_view(`cfg_main`);
                                        return;
                                    }
                                    await dialog.alert(
                                        `${$t(`icu.invalid_*`, { value: `${$t(`common.key`)}`.toLowerCase() })}`,
                                    );
                                }
                                break;
                        }
                    }
                    break;
                case `cfg_main`:
                    {
                        switch ($carousel_index) {
                            case 0:
                                {
                                    if (cfg_main_profilename_loading) return;
                                    const ks_nostr_secretkey =
                                        await keystore.get(
                                            ks.cfg_init.nostr_secretkey,
                                        );
                                    if (`err` in ks_nostr_secretkey) {
                                        await reset_page(
                                            `${$t(`error.device.configuration_failure`)}`,
                                        );
                                        return;
                                    }
                                    const kv_profile_name = await kv.get(
                                        fmt_id(page_param.kv.nostr_profilename),
                                    );
                                    if (!kv_profile_name) {
                                        await dialog.alert(
                                            `${$t(`icu.enter_a_*`, { value: `${$t(`common.profile_name`)}`.toLowerCase() })}`,
                                        );
                                        return;
                                    }

                                    cfg_main_profilename_loading = true;
                                    const profilename_validated =
                                        await fetch_radroots_profile_validate({
                                            profile_name: kv_profile_name,
                                        });
                                    if (`err` in profilename_validated) {
                                        cfg_main_profilename_loading = false;
                                        await dialog.alert(
                                            profilename_validated.err,
                                        );
                                        return;
                                    }
                                    const confirm = await dialog.confirm({
                                        message: `${`${$t(`icu.the_*_is_available`, { value: `${$t(`common.profile_name`).toLowerCase()} "${profilename_validated.profile_name}"` })}`}. Would you like to use it?`, //@todo
                                        cancel_label: `${$t(`common.no`)}`,
                                        ok_label: `${$t(`common.yes`)}`,
                                    });
                                    if (!confirm) {
                                        cfg_main_profilename_loading = false;
                                        return;
                                    }
                                    const profilename_added =
                                        await fetch_radroots_profile_init({
                                            profile_name:
                                                profilename_validated.profile_name,
                                            secret_key:
                                                ks_nostr_secretkey.result,
                                        });
                                    cfg_main_profilename_loading = false;
                                    if (`err` in profilename_added) {
                                        await dialog.alert(
                                            profilename_added.err,
                                        );
                                        return;
                                    }
                                    await keystore.set(
                                        ks.cfg_init.nostr_profilename,
                                        profilename_validated.profile_name,
                                    );
                                    await keystore.set(
                                        ks.cfg_init.radroots_tok,
                                        profilename_added.tok,
                                    );

                                    carousel_next(view);
                                }
                                break;
                            case 1: {
                                if (!cfg_main_opt_idx1)
                                    cfg_main_opt_idx1 = `personal`;
                                await keystore.set(
                                    ks.pref.cfg_type,
                                    cfg_main_opt_idx1,
                                );
                                handle_view(`eula`);
                            }
                        }
                    }
                    break;
            }
        } catch (e) {
            console.log(`(error) handle_continue `, e);
        }
    };

    const fetch_radroots_profile_validate = async (opts: {
        profile_name: string;
    }): Promise<{ profile_name: string } | ErrorMessage<string>> => {
        try {
            const res = await http.fetch({
                url: `${PUBLIC_RADROOTS_URL}/public/accounts/list`,
                method: `post`,
            });
            console.log(JSON.stringify(res, null, 4), `res`);
            if (`err` in res)
                return err_msg(`${$t(`error.client.network_failure`)}`);
            else if (Array.isArray(res.data.results)) {
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
            console.log(`(error) fetch_radroots_profile_validate `, e);
            return err_msg(`${$t(`error.client.network_failure`)}`);
        }
    };

    const fetch_radroots_profile_init = async (opts: {
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
            console.log(JSON.stringify(res, null, 4), `res`);
            if (`err` in res) return res;
            else if (res.data && `tok` in res.data) {
                return { tok: res.data.tok };
            } else if (res.data && `message` in res.data)
                return err_msg(
                    `${$t(`radroots-org.error.${res.data.message}`, { default: `${$t(`error.client.request_failure`)}` })}`,
                );
            return err_msg(`${$t(`error.client.request_failure`)}`);
        } catch (e) {
            console.log(`(error) fetch_radroots_profile_init `, e);
            return err_msg(`${$t(`error.client.network_failure`)}`);
        }
    };

    const fetch_radroots_profile_confirm = async (
        authorization: string,
    ): Promise<{ pass: true } | ErrorMessage<string>> => {
        try {
            const res = await http.fetch({
                url: `${PUBLIC_RADROOTS_URL}/public/accounts/add/conf`,
                method: `post`,
                authorization,
            });
            console.log(JSON.stringify(res, null, 4), `res`);
            if (`err` in res) return res;
            return { pass: true };
        } catch (e) {
            console.log(`(error) fetch_radroots_profile_confirm `, e);
            return err_msg(`${$t(`error.client.network_failure`)}`);
        }
    };

    const fetch_radroots_profile_status = async (
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
            console.log(JSON.stringify(res, null, 4), `res`);
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
            console.log(`(error) fetch_radroots_profile_confirm `, e);
            return err_msg(`${$t(`error.client.network_failure`)}`);
        }
    };

    const submit = async (): Promise<void> => {
        try {
            const ks_nostr_secretkey = await keystore.get(
                ks.cfg_init.nostr_secretkey,
            );
            console.log(
                JSON.stringify(ks_nostr_secretkey, null, 4),
                `ks_nostr_secretkey`,
            );
            if (`err` in ks_nostr_secretkey) {
                await dialog.alert(
                    `${$t(`error.device.configuration_failure`)}`,
                );
                return; //@todo
            }
            const ks_radroots_tok = await keystore.get(
                ks.cfg_init.radroots_tok,
            );
            if (`result` in ks_radroots_tok) {
                const profile_activated = await fetch_radroots_profile_confirm(
                    ks_radroots_tok.result,
                );
                if (`err` in profile_activated) {
                    await dialog.alert(
                        `${$t(`icu.*_failure`, { value: `${$t(`common.activation`)}` })}`,
                    );
                    return; //@todo
                }
            }
            const ks_nostr_profilename = await keystore.get(
                ks.cfg_init.nostr_profilename,
            );
            const nostr_publickey = nostr.lib.public_key(
                ks_nostr_secretkey.result,
            );
            if (!nostr_publickey) {
                await dialog.alert(
                    `${$t(`error.device.public_key_not_derived`)}`,
                );
                return; //@todo
            }
            const nostr_profile_add = await db.nostr_profile_add({
                public_key: nostr_publickey,
                name:
                    `result` in ks_nostr_profilename
                        ? ks_nostr_profilename.result
                        : undefined,
            });

            if (`err` in nostr_profile_add || `err_s` in nostr_profile_add) {
                await dialog.alert(
                    `${$t(`icu.failure_saving_*_to_the_database`, { value: `${$t(`common.profile`)}`.toLowerCase() })}`,
                );
                return; //@todo
            }
            await keystore.set(ks.keys.nostr_publickey, nostr_publickey);
            await keystore.set(
                ks.keys.nostr_secretkey(nostr_publickey),
                ks_nostr_secretkey.result,
            );
            for (const url of Array.from(
                new Set([
                    cfg.nostr.relay_url,
                    ...PUBLIC_NOSTR_RELAY_DEFAULTS.split(","),
                ]),
            )) {
                const nostr_relay_add = await db.nostr_relay_add({ url });
                if (`err` in nostr_relay_add || `err_s` in nostr_relay_add) {
                    await dialog.alert(
                        `${$t(`icu.failure_saving_*_to_the_database`, { value: `${$t(`icu.default_*`, { value: `${$t(`common.relays`)}` })}`.toLowerCase() })}`,
                    );
                    return; // @todo
                }
                const nostr_profile_relay_add =
                    await db.set_nostr_profile_relay({
                        nostr_profile: {
                            id: nostr_profile_add.id,
                        },
                        nostr_relay: {
                            id: nostr_relay_add.id,
                        },
                    });
                console.log(
                    JSON.stringify(nostr_profile_relay_add, null, 4),
                    `nostr_profile_relay_add`,
                );
            }
            await reset_ks();
            await restart({
                route: `/`,
                notify_message: `${$t(`app.page.cfg.init.notification.welcome`)}`,
            });
        } catch (e) {
            console.log(`(error) submit `, e);
        }
    };
</script>

<LayoutView>
    <div
        data-view={`cfg_init`}
        class={`flex flex-col h-full w-full max-mobile_base:pt-12 pt-16 justify-start items-center`}
    >
        <div
            data-carousel-container={`cfg_init`}
            class={`carousel-container flex h-full w-full`}
        >
            <div
                data-carousel-item={`cfg_init`}
                class={`carousel-item flex flex-col w-full max-mobile_y:pt-28 pt-32 pb-4 justify-start items-center`}
            >
                <div class={`flex flex-col gap-8 justify-start items-center`}>
                    <div
                        class={`flex flex-col gap-1 justify-start items-center`}
                    >
                        <button
                            class={`flex flex-row justify-center items-center`}
                            on:click={async () => {
                                await route(`/`);
                            }}
                        >
                            <p
                                class={`font-mono font-[700] text-layer-0-glyph text-4xl`}
                            >
                                {`${`${$t(`app.name`)}`}`}
                            </p>
                        </button>
                        <button
                            class={`flex flex-row justify-center items-center`}
                            on:click={async () => {
                                const sk = nostr.lib.generate_key();
                                const res1 = await db.nostr_profile_add({
                                    public_key: nostr.lib.public_key(sk),
                                });
                                console.log(
                                    JSON.stringify(res1, null, 4),
                                    `res1`,
                                );
                            }}
                        >
                            <p
                                class={`font-mono font-[700] text-layer-0-glyph text-4xl`}
                            >
                                {`${`${$t(`common.setup`)}`}`}
                            </p>
                        </button>
                    </div>
                    <div
                        class={`grid grid-cols-12 flex flex-col gap-4 w-full justify-start items-center`}
                    >
                        {#each [`Configure your device`, `Choose a profile name`, `Terms of Use agreement`] as li, li_i}
                            <div
                                class={`col-span-12 flex flex-row justify-start items-center`}
                            >
                                <p
                                    class={`font-mono font-[400] text-layer-0-glyph text-xl`}
                                >
                                    {`${li_i + 1}. ${li}`}
                                </p>
                            </div>
                        {/each}
                    </div>
                </div>
            </div>
            <button
                data-carousel-item={`cfg_init`}
                class={`carousel-item flex flex-col w-full max-mobile_y:pt-32 pt-36 pb-4 justify-start items-center`}
                on:click={async () => {
                    cgf_init_key_option = undefined;
                }}
            >
                <div
                    class={`flex flex-col w-full gap-10 justify-start items-center`}
                >
                    <div
                        class={`flex flex-row w-full justify-center items-center`}
                    >
                        <p
                            class={`font-mono font-[600] text-layer-0-glyph text-3xl`}
                        >
                            {`${$t(`icu.configure_*`, { value: `${$t(`common.device`)}` })}`}
                        </p>
                    </div>
                    <div
                        class={`flex flex-col w-full gap-5 justify-center items-center`}
                    >
                        <button
                            class={`flex flex-col h-touch_bold w-${$app_layout} justify-center items-center bg-layer-1-surface rounded-3xl touch-layer-1 touch-layer-1-raise-less ${cgf_init_key_option === `key_gen` ? `layer-1-ring` : ``}`}
                            on:click|stopPropagation={async () => {
                                cgf_init_key_option = `key_gen`;
                            }}
                        >
                            <p
                                class={`font-sans font-[600] text-layer-0-glyph text-xl`}
                            >
                                {`${$t(`icu.create_new_*`, { value: `${$t(`common.keypair`)}`.toLowerCase() })}`}
                            </p>
                        </button>
                        <button
                            class={`flex flex-col h-touch_bold w-${$app_layout} justify-center items-center bg-layer-1-surface rounded-3xl touch-layer-1 touch-layer-1-raise-less ${cgf_init_key_option === `kv_nostr_secretkey` ? `layer-1-ring` : ``}`}
                            on:click|stopPropagation={async () => {
                                cgf_init_key_option = `kv_nostr_secretkey`;
                            }}
                        >
                            <p
                                class={`font-sans font-[600] text-layer-0-glyph text-xl`}
                            >
                                {`${$t(`icu.use_existing_*`, {
                                    value: `${$t(`common.keypair`)}`.toLowerCase(),
                                })}`}
                            </p>
                        </button>
                    </div>
                </div>
            </button>
            <div
                data-carousel-item={`cfg_init`}
                class={`carousel-item flex flex-col w-full max-mobile_y:pt-32 pt-36 pb-4 justify-start items-center`}
            >
                <div
                    class={`flex flex-col w-full gap-8 justify-start items-center`}
                >
                    <div
                        class={`flex flex-col w-full gap-6 justify-center items-center`}
                    >
                        {#if cfg_main_nostr_publickey}
                            <p
                                class={`font-mono font-[600] text-layer-0-glyph text-3xl`}
                            >
                                {`Using Public Key`}
                            </p>
                            <DisplayLine
                                basis={{
                                    classes: `w-${$app_layout}`,
                                    label: {
                                        classes: `pl-4 font-mono text-lg text-start truncate`,
                                        value:
                                            cfg_main_nostr_publickey_npub ||
                                            cfg_main_nostr_publickey,
                                    },
                                    style: `guide`,
                                }}
                            />
                        {:else}
                            <p
                                class={`font-mono font-[600] text-layer-0-glyph text-3xl`}
                            >
                                {`${$t(`icu.add_existing_*`, { value: `${$t(`common.key`)}`.toLowerCase() })}`}
                            </p>
                            <InputElement
                                basis={{
                                    classes: `h-entry_guide w-${$app_layout} bg-layer-1-surface rounded-touch font-mono text-lg placeholder:opacity-60 items-end text-center`,
                                    id: fmt_id(page_param.kv.nostr_secretkey),
                                    sync: true,
                                    sync_init: true,
                                    placeholder: `${$t(`icu.enter_*`, { value: `nostr nsec/hex` })}`,
                                    field: {
                                        charset: regex.profile_name_ch,
                                        validate: regex.profile_name,
                                        validate_keypress: true,
                                    },
                                    callback_keydown: async ({ key, el }) => {
                                        if (key === `Enter`) {
                                            el.blur();
                                            await handle_continue();
                                        }
                                    },
                                }}
                            />
                        {/if}
                    </div>
                </div>
            </div>
        </div>
        <div
            class={`absolute max-mobile_base:bottom-0 bottom-8 left-0 flex flex-col w-full justify-center items-center`}
        >
            <ButtonCarouselPair
                basis={{
                    continue: {
                        disabled: $carousel_index === 1 && !cgf_init_key_option,
                        callback: async () => await handle_continue(),
                    },
                    back: {
                        visible: $carousel_index > 0,
                        callback: async () => await handle_back(),
                    },
                }}
            />
        </div>
    </div>
    <div
        data-view={`cfg_main`}
        class={`hidden flex flex-col h-full w-full max-mobile_base:pt-12 pt-16 justify-start items-center`}
    >
        <div
            data-carousel-container={`cfg_main`}
            class={`carousel-container flex h-full w-full`}
        >
            <div
                data-carousel-item={`cfg_main`}
                class={`carousel-item flex flex-col w-full max-mobile_y:pt-32 pt-36 pb-4 justify-start items-center`}
            >
                <div
                    class={`flex flex-col w-full gap-8 justify-start items-center`}
                >
                    <div
                        class={`flex flex-col w-full gap-6 justify-center items-center`}
                    >
                        <p
                            class={`font-mono font-[600] text-layer-0-glyph text-3xl`}
                        >
                            {`${$t(`icu.add_*`, { value: `${$t(`common.profile`)}` })}`}
                        </p>
                        <EntryLine
                            basis={{
                                loading: cfg_main_profilename_loading,
                                classes: `w-${$app_layout}`,
                                style: `guide`,
                                el: {
                                    classes: `font-mono text-lg text-center placeholder:opacity-60`,
                                    id: fmt_id(page_param.kv.nostr_profilename),
                                    sync: true,
                                    sync_init: true,
                                    placeholder: `${$t(`icu.enter_*`, { value: `${$t(`common.profile_name`)}`.toLowerCase() })}`,
                                    field: {
                                        charset: regex.profile_name_ch,
                                        validate: regex.profile_name,
                                        validate_keypress: true,
                                    },
                                    callback: async ({ pass }) => {
                                        cfg_main_profilename_valid = pass;
                                    },
                                    callback_keydown: async ({ key, el }) => {
                                        if (key === `Enter`) {
                                            el.blur();
                                            await handle_continue();
                                        }
                                    },
                                },
                            }}
                        />
                    </div>
                </div>
            </div>
            <button
                data-carousel-item={`cfg_main`}
                class={`carousel-item flex flex-col w-full max-mobile_y:pt-32 pt-36 pb-4 justify-start items-center`}
                on:click={async () => {
                    cfg_main_opt_idx1 = undefined;
                }}
            >
                <div
                    class={`flex flex-col w-full gap-10 justify-start items-center`}
                >
                    <div
                        class={`flex flex-row w-full justify-center items-center`}
                    >
                        <p
                            class={`font-mono font-[600] text-layer-0-glyph text-3xl`}
                        >
                            {`${$t(`common.setup_for_farmer`)}`}
                        </p>
                    </div>
                    <div
                        class={`flex flex-col w-full gap-5 justify-center items-center`}
                    >
                        <button
                            class={`flex flex-col h-touch_bold w-${$app_layout} justify-center items-center bg-layer-1-surface rounded-3xl touch-layer-1 touch-layer-1-raise-less ${cfg_main_opt_idx1 === `farmer` ? `layer-1-ring` : ``}`}
                            on:click|stopPropagation={async () => {
                                cfg_main_opt_idx1 = `farmer`;
                            }}
                        >
                            <p
                                class={`font-sans font-[600] text-layer-0-glyph text-xl`}
                            >
                                {`${$t(`common.yes`)}`}
                            </p>
                        </button>
                        <button
                            class={`flex flex-col h-touch_bold w-${$app_layout} justify-center items-center bg-layer-1-surface rounded-3xl touch-layer-1 touch-layer-1-raise-less ${cfg_main_opt_idx1 === `personal` ? `layer-1-ring` : ``}`}
                            on:click|stopPropagation={async () => {
                                cfg_main_opt_idx1 = `personal`;
                            }}
                        >
                            <p
                                class={`font-sans font-[600] text-layer-0-glyph text-xl`}
                            >
                                {`${$t(`common.no`)}`}
                            </p>
                        </button>
                    </div>
                </div>
            </button>
        </div>
        <div
            class={`absolute top-16 left-4 flex flex-row gap-2 justify-start items-center ${view === `cfg_main` ? `fade-in-long` : ``}`}
        >
            <button
                class={`group flex flex-row justify-center items-center`}
                on:click={async () => {
                    await handle_back();
                }}
            >
                <Glyph
                    basis={{
                        classes: `text-layer-1-glyph-shade group-active:opacity-60 transition-all`,
                        key: `caret-left`,
                        dim: `lg`,
                        weight: `bold`,
                    }}
                />
                <p
                    class={`font-sans font-[400] text-layer-0-glyph text-lg capitalize group-active:opacity-60 transition-all`}
                >
                    {`${$t(`icu.go_*`, { value: `${$t(`common.back`)}` })}`}
                </p>
            </button>
        </div>
        <div
            class={`absolute max-mobile_base:bottom-0 bottom-8 left-0 flex flex-col w-full justify-center items-center ${view === `cfg_main` ? `fade-in-long` : ``}`}
        >
            <ButtonCarouselPair
                basis={{
                    continue: {
                        disabled:
                            ($carousel_index === 0 &&
                                !cfg_main_profilename_valid) ||
                            ($carousel_index === 1 && !cfg_main_opt_idx1),
                        callback: async () => await handle_continue(),
                    },
                    back: {
                        visible: true,
                        label:
                            $carousel_index === 0
                                ? `${$t(`common.skip`)}`
                                : `${$t(`common.back`)}`,
                        callback: async () => {
                            if ($carousel_index === 0) {
                                const confirm = await dialog.confirm({
                                    message: `${$t(`app.page.cfg.init.notification.no_profile_name`)}`,
                                    cancel_label: `${$t(`icu.add_*`, { value: `${$t(`common.profile`)}` })}`,
                                    ok_label: `${$t(`common.continue`)}`,
                                });
                                if (confirm === false) {
                                    el_id(
                                        fmt_id(page_param.kv.nostr_profilename),
                                    )?.focus();
                                    return;
                                }
                                carousel_next(view);
                                return;
                            }
                            await handle_back();
                        },
                    },
                }}
            />
        </div>
    </div>
    <div
        data-view={`eula`}
        class={`hidden flex flex-col h-full w-full max-mobile_base:pt-12 pt-12 justify-start items-center`}
    >
        <div
            data-carousel-container={`eula`}
            class={`carousel-container flex h-full w-full rounded-2xl scroll-hide`}
        >
            <div
                data-carousel-item={`eula`}
                class={`carousel-item flex flex-col w-full max-mobile_base:pt-16 justify-start items-center`}
            >
                <div
                    class={`flex flex-col w-full px-4 pb-2 justify-start items-center ${view === `eula` ? `fade-in-long` : ``} overflow-hidden`}
                >
                    <div
                        class={`flex flex-col w-full px-4 gap-4 justify-start items-center`}
                    >
                        <div
                            class={`flex flex-row w-full justify-center items-center`}
                        >
                            <p
                                class={`font-mono font-[600] text-layer-0-glyph text-2xl`}
                            >
                                {`${$t(`eula.title`)}`}
                            </p>
                        </div>
                        <div
                            bind:this={el_eula}
                            class={`flex flex-col h-[34rem] w-full gap-6 justify-start items-center overflow-y-scroll scroll-hide`}
                        >
                            <div
                                class={`flex flex-col w-full gap-2 justify-start items-start`}
                            >
                                <p
                                    class={`font-mono font-[600] text-layer-0-glyph`}
                                >
                                    {`**${$t(`eula.introduction.title`)}**`}
                                </p>
                                <p
                                    class={`font-mono font-[500] text-layer-0-glyph text-sm text-justify break-word`}
                                >
                                    {`${$t(`eula.introduction.body`)}`}
                                </p>
                            </div>
                            <div
                                class={`flex flex-col w-full gap-2 justify-start items-start`}
                            >
                                <p
                                    class={`font-mono font-[600] text-layer-0-glyph`}
                                >
                                    {`**${$t(`eula.prohibited_content.title`)}**`}
                                </p>
                                <p
                                    class={`font-mono font-[500] text-sm text-layer-0-glyph text-justify break-word`}
                                >
                                    {`${$t(`eula.prohibited_content.body_0_title`)}`}
                                </p>
                                <div
                                    class={`flex flex-col w-full justify-start items-start`}
                                >
                                    {#each [0, 1, 2, 3, 4, 5] as li}
                                        <div
                                            class={`flex flex-row w-full justify-start items-center`}
                                        >
                                            <div
                                                class={`flex flex-row h-full w-8 justify-start items-start`}
                                            >
                                                <p
                                                    class={` font-mono font-[500] text-sm text-layer-0-glyph text-justify break-word`}
                                                >
                                                    {`*`}
                                                </p>
                                            </div>
                                            <div
                                                class={`flex flex-row h-full w-full justify-start items-start`}
                                            >
                                                <p
                                                    class={`col-span-10 font-mono font-[500] text-sm text-layer-0-glyph text-justify break-word`}
                                                >
                                                    {`${$t(`eula.prohibited_content.body_li_0_${li}`)}`}
                                                </p>
                                            </div>
                                        </div>
                                    {/each}
                                </div>
                            </div>
                            <div
                                class={`flex flex-col w-full gap-2 justify-start items-start`}
                            >
                                <p
                                    class={`font-mono font-[600] text-layer-0-glyph`}
                                >
                                    {`**${$t(`eula.prohibited_conduct.title`)}**`}
                                </p>
                                <div
                                    class={`flex flex-col w-full justify-start items-start`}
                                >
                                    {#each [0, 1, 2, 3] as li}
                                        <div
                                            class={`flex flex-row w-full justify-start items-center`}
                                        >
                                            <div
                                                class={`flex flex-row h-full w-8 justify-start items-start`}
                                            >
                                                <p
                                                    class={` font-mono font-[500] text-sm text-layer-0-glyph text-justify break-word`}
                                                >
                                                    {`*`}
                                                </p>
                                            </div>
                                            <div
                                                class={`flex flex-row h-full w-full justify-start items-start`}
                                            >
                                                <p
                                                    class={`col-span-10 font-mono font-[500] text-sm text-layer-0-glyph text-justify break-word`}
                                                >
                                                    {`${$t(`eula.prohibited_conduct.body_li_0_${li}`)}`}
                                                </p>
                                            </div>
                                        </div>
                                    {/each}
                                </div>
                            </div>
                            <div
                                class={`flex flex-col w-full gap-2 justify-start items-start`}
                            >
                                <p
                                    class={`font-mono font-[600] text-layer-0-glyph`}
                                >
                                    {`**${$t(`eula.consequences_of_violation.title`)}**`}
                                </p>
                                <p
                                    class={`font-mono font-[500] text-layer-0-glyph text-sm text-justify break-word`}
                                >
                                    {`${$t(`eula.consequences_of_violation.body`)}`}
                                </p>
                            </div>
                            <div
                                class={`flex flex-col w-full gap-2 justify-start items-start`}
                            >
                                <p
                                    class={`font-mono font-[600] text-layer-0-glyph`}
                                >
                                    {`**${$t(`eula.disclaimer.title`)}**`}
                                </p>
                                <p
                                    class={`font-mono font-[500] text-layer-0-glyph text-sm text-justify break-word`}
                                >
                                    {`${$t(`eula.disclaimer.body`)}`}
                                </p>
                            </div>
                            <div
                                class={`flex flex-col w-full gap-2 justify-start items-start`}
                            >
                                <p
                                    class={`font-mono font-[600] text-layer-0-glyph`}
                                >
                                    {`**${$t(`eula.changes.title`)}**`}
                                </p>
                                <p
                                    class={`font-mono font-[500] text-layer-0-glyph text-sm text-justify break-word`}
                                >
                                    {`${$t(`eula.changes.body`)}`}
                                </p>
                            </div>
                            <div
                                class={`flex flex-col w-full gap-2 justify-start items-start`}
                            >
                                <p
                                    class={`font-mono font-[600] text-layer-0-glyph`}
                                >
                                    {`**${$t(`eula.contact.title`)}**`}
                                </p>
                                <p
                                    class={`font-mono font-[500] text-layer-0-glyph text-sm text-justify break-word`}
                                >
                                    {`${$t(`eula.contact.body`)}`}
                                </p>
                            </div>
                            <div
                                class={`flex flex-col w-full gap-2 justify-start items-start`}
                            >
                                <p
                                    class={`font-mono font-[600] text-layer-0-glyph`}
                                >
                                    {`**${$t(`eula.acceptance_of_terms.title`)}**`}
                                </p>
                                <p
                                    class={`font-mono font-[500] text-layer-0-glyph text-sm text-justify break-word`}
                                >
                                    {`${$t(`eula.acceptance_of_terms.body`)}`}
                                </p>
                            </div>
                        </div>
                    </div>
                    <div
                        class={`flex flex-row w-full pt-8 justify-center items-center`}
                    >
                        <button
                            class={`group flex flex-row basis-1/2 gap-4 justify-center items-center`}
                            on:click={async () => {
                                const confirm = await dialog.confirm({
                                    message: `${$t(`eula.error.required`)}`,
                                    cancel_label: `${$t(`common.quit`)}`,
                                });
                                if (confirm === false) location.reload(); //@todo
                            }}
                        >
                            <p
                                class={`font-mono font-[400] text-sm text-layer-0-glyph/60 group-active:text-layer-0-glyph transition-all`}
                            >
                                {`-`}
                            </p>
                            <p
                                class={`font-mono font-[400] text-sm text-layer-0-glyph/60 group-active:text-layer-0-glyph transition-all`}
                            >
                                {`${`${$t(`common.disagree`)}`}`}
                            </p>
                            <p
                                class={`font-mono font-[400] text-sm text-layer-0-glyph/60 group-active:text-layer-0-glyph transition-all`}
                            >
                                {`-`}
                            </p>
                        </button>
                        <button
                            class={`group flex flex-row basis-1/2 gap-4 justify-center items-center ${el_eula_scrolled ? `` : `opacity-40`}`}
                            on:click={async () => {
                                if (el_eula_scrolled) await submit();
                            }}
                        >
                            <p
                                class={`font-mono font-[400] text-sm text-layer-0-glyph-hl group-active:text-layer-0-glyph-hl/80 transition-all`}
                            >
                                {`-`}
                            </p>
                            <p
                                class={`font-mono font-[400] text-sm text-layer-0-glyph-hl group-active:text-layer-0-glyph-hl/80 transition-all`}
                            >
                                {`${`${$t(`common.agree`)}`}`}
                            </p>
                            <p
                                class={`font-mono font-[400] text-sm text-layer-0-glyph-hl group-active:text-layer-0-glyph-hl/80 transition-all`}
                            >
                                {`- `}
                            </p>
                        </button>
                    </div>
                </div>
            </div>
        </div>
    </div>
</LayoutView>
