<script lang="ts">
    import { PUBLIC_NOSTR_RELAY_DEFAULTS } from "$env/static/public";
    import { db, dialog, keystore, nostr } from "$lib/client";
    import { cfg, ks } from "$lib/conf";
    import { restart } from "$lib/util/client";
    import {
        fetch_radroots_profile_confirm,
        fetch_radroots_profile_init,
        fetch_radroots_profile_status,
        fetch_radroots_profile_validate,
    } from "$lib/util/fetch-radroots-profile";
    import { kv_init_page } from "$lib/util/kv";
    import type { IClientUnlisten } from "@radroots/client";
    import {
        app_layout,
        app_loading,
        app_splash,
        ButtonLayoutPair,
        carousel_dec,
        carousel_inc,
        carousel_index,
        carousel_index_max,
        DisplayLine,
        el_id,
        EntryLine,
        fmt_id,
        Glyph,
        InputElement,
        kv,
        LayoutView,
        Loading,
        LogoCircle,
        ls,
        sleep,
        view_effect,
    } from "@radroots/svelte-lib";
    import { regex } from "@radroots/utils";
    import { onDestroy, onMount } from "svelte";

    const page_carousel: Record<View, { max_index: number }> = {
        cfg_init: {
            max_index: 2,
        },
        cfg_main: {
            max_index: 2,
        },
        eula: {
            max_index: 1,
        },
    };

    let el_eula: HTMLDivElement;
    let el_eula_scrolled = false;

    let loading_submit = false;

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
            await init_page();
        } catch (e) {
            console.log(`e mount`, e);
        } finally {
            app_splash.set(false);
        }
    });

    onDestroy(async () => {
        try {
            await kv_init_page();
            if (unlisten_cfg_init_nostr_secretkey)
                unlisten_cfg_init_nostr_secretkey();
            el_eula?.removeEventListener(`scroll`, () => null);
        } catch (e) {
            console.log(`e destroy`, e);
        } finally {
        }
    });

    const init_page = async (): Promise<void> => {
        try {
            handle_view(initial_view);
            const unlisten_1 = await keystore.on_key_change(
                ks.cfg_init.nostr_secretkey,
                async (_cfg_init_nostr_secretkey) => {
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
            await kv_init_page();
        } catch (e) {
            console.log(`(error) init_page `, e);
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
                            message: `There is an existing configuration in progress${`nip_05` in profile_status.active ? ` for "${profile_status.active.nip_05}".` : `.`} ${`${$ls(`common.do_you_want_to_continue_q`)}`}`, //@todo
                            cancel_label: `${$ls(`common.reset`)}`,
                            ok_label: `${$ls(`common.yes`)}`,
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
                                    fmt_id(`nostr_profilename`),
                                    profile_status.active.nip_05,
                                );
                            }
                            await kv.set(
                                fmt_id(`nostr_profilename`),
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

    const handle_view = (new_view: View): void => {
        if (new_view === `cfg_init` && view === `cfg_main`) {
            const offset = cgf_init_key_option === `key_gen` ? 1 : 0;
            carousel_index.set(page_carousel[new_view].max_index - offset);
        } else {
            carousel_index.set(0);
            carousel_index_max.set(page_carousel[new_view].max_index);
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
                                    await carousel_dec(view);
                                }
                                break;
                            case 2:
                                {
                                    await carousel_dec(view);
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
                                carousel_dec(view);
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
                                    await carousel_inc(view);
                                }
                                break;
                            case 1:
                                {
                                    if (
                                        cgf_init_key_option ===
                                        `kv_nostr_secretkey`
                                    ) {
                                        await carousel_inc(view);
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
                                        fmt_id(`nostr_secretkey`),
                                    );
                                    if (!kv_nostr_secretkey) {
                                        await dialog.alert(
                                            `${$ls(`icu.enter_a_*`, { value: `${$ls(`icu.valid_*`, { value: `${$ls(`common.key`)}` })}`.toLowerCase() })}`,
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
                                        `${$ls(`icu.invalid_*`, { value: `${$ls(`common.key`)}`.toLowerCase() })}`,
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
                                            `${$ls(`error.device.configuration_failure`)}`,
                                        );
                                        return;
                                    }
                                    const kv_profile_name = await kv.get(
                                        fmt_id(`nostr_profilename`),
                                    );
                                    if (!kv_profile_name) {
                                        await dialog.alert(
                                            `${$ls(`icu.enter_a_*`, { value: `${$ls(`common.profile_name`)}`.toLowerCase() })}`,
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
                                        message: `${`${$ls(`icu.the_*_is_available`, { value: `${$ls(`common.profile_name`).toLowerCase()} "${profilename_validated.profile_name}"` })}`}. Would you like to use it?`, //@todo
                                        cancel_label: `${$ls(`common.no`)}`,
                                        ok_label: `${$ls(`common.yes`)}`,
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

                                    carousel_inc(view);
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

    const submit = async (): Promise<void> => {
        try {
            loading_submit = true;

            const ks_nostr_secretkey = await keystore.get(
                ks.cfg_init.nostr_secretkey,
            );
            if (`err` in ks_nostr_secretkey) {
                await dialog.alert(
                    `${$ls(`error.device.configuration_failure`)}`,
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
                        `${$ls(`icu.*_failure`, { value: `${$ls(`common.activation`)}` })}`,
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
                    `${$ls(`error.device.public_key_not_derived`)}`,
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
                    `${$ls(`icu.failure_saving_*_to_the_database`, { value: `${$ls(`common.profile`)}`.toLowerCase() })}`,
                );
                return; //@todo
            }
            await keystore.set(ks.keys.nostr_publickey, nostr_publickey);
            await keystore.set(
                ks.keys.nostr_secretkey(nostr_publickey),
                ks_nostr_secretkey.result,
            );
            for (const url of Array.from(
                new Set([...PUBLIC_NOSTR_RELAY_DEFAULTS.split(",")]),
            )) {
                const nostr_relay_add = await db.nostr_relay_add({ url });
                if (`err` in nostr_relay_add || `err_s` in nostr_relay_add) {
                    await dialog.alert(
                        `${$ls(`icu.failure_saving_*_to_the_database`, { value: `${$ls(`icu.default_*`, { value: `${$ls(`common.relays`)}` })}`.toLowerCase() })}`,
                    );
                    return; // @todo
                }
                await db.nostr_profile_relay_set({
                    nostr_profile: {
                        id: nostr_profile_add.id,
                    },
                    nostr_relay: {
                        id: nostr_relay_add.id,
                    },
                });
            }
            await reset_ks();
            await restart({
                route: `/`,
                notify_message: `${$ls(`app.cfg.init.notification.welcome`)}`,
            });
        } catch (e) {
            console.log(`(error) submit `, e);
        } finally {
            loading_submit = false;
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
                class={`carousel-item flex flex-col w-full max-mobile_y:pt-28 pt-36 pb-4 justify-start items-center`}
            >
                <div class={`flex flex-col gap-1 justify-start items-center`}>
                    <div
                        class={`flex flex-row w-full justify-center items-center`}
                    >
                        <LogoCircle />
                    </div>
                    <div
                        class={`flex flex-col w-full pt-4 px-12 gap-2 justify-start items-center`}
                    >
                        <div
                            class={`flex flex-row w-full justify-start items-center`}
                        >
                            <p
                                class={`font-sans font-[400] text-sm text-layer-0-glyph-label uppercase`}
                            >
                                {`${$ls(`common.configure`)}`}
                            </p>
                        </div>
                        <div
                            class={`flex flex-col w-full gap-2 justify-start items-center`}
                        >
                            <div
                                class={`flex flex-row w-full justify-start items-center`}
                            >
                                <p
                                    class={`font-mono font-[400] text-[1.1rem] text-layer-0-glyph`}
                                >
                                    {`${$ls(`app.cfg.init.greeting`)}`}
                                </p>
                            </div>
                            <div
                                class={`flex flex-row w-full justify-start items-center`}
                            >
                                <p
                                    class={`font-mono font-[400] text-[1.1rem] text-layer-0-glyph`}
                                >
                                    {`${$ls(`app.cfg.init.message`)}`}
                                </p>
                            </div>
                        </div>
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
                            {`${$ls(`icu.configure_*`, { value: `${$ls(`common.device`)}` })}`}
                        </p>
                    </div>
                    <div
                        class={`flex flex-col w-full gap-5 justify-center items-center`}
                    >
                        <button
                            class={`flex flex-col h-touch_bold w-${$app_layout} justify-center items-center rounded-touch ${cgf_init_key_option === `key_gen` ? `layer-1-surface-apply-active layer-1-raise-apply layer-1-ring-apply` : `bg-layer-1-surface`} el-re`}
                            on:click|stopPropagation={async () => {
                                cgf_init_key_option = `key_gen`;
                            }}
                        >
                            <p
                                class={`font-sans font-[600] text-layer-0-glyph text-xl`}
                            >
                                {`${$ls(`icu.create_new_*`, { value: `${$ls(`common.keypair`)}`.toLowerCase() })}`}
                            </p>
                        </button>
                        <button
                            class={`flex flex-col h-touch_bold w-${$app_layout} justify-center items-center rounded-touch ${cgf_init_key_option === `kv_nostr_secretkey` ? `layer-1-surface-apply-active layer-1-raise-apply layer-1-ring-apply` : `bg-layer-1-surface`} el-re`}
                            on:click|stopPropagation={async () => {
                                cgf_init_key_option = `kv_nostr_secretkey`;
                            }}
                        >
                            <p
                                class={`font-sans font-[600] text-layer-0-glyph text-xl`}
                            >
                                {`${$ls(`icu.use_existing_*`, {
                                    value: `${$ls(`common.keypair`)}`.toLowerCase(),
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
                                {`${$ls(`common.using_public_key`)}`}
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
                                {`${$ls(`icu.add_existing_*`, { value: `${$ls(`common.key`)}`.toLowerCase() })}`}
                            </p>
                            <InputElement
                                basis={{
                                    classes: `h-entry_guide w-${$app_layout} bg-layer-1-surface rounded-touch font-mono text-lg placeholder:opacity-60 items-end text-center`,
                                    id: fmt_id(`nostr_secretkey`),
                                    sync: true,
                                    placeholder: `${$ls(`icu.enter_*`, { value: `nostr nsec/hex` })}`,
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
            <ButtonLayoutPair
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
                            {`${$ls(`icu.add_*`, { value: `${$ls(`common.profile`)}` })}`}
                        </p>
                        <EntryLine
                            basis={{
                                loading: cfg_main_profilename_loading,
                                wrap: {
                                    layer: 1,
                                    classes: `w-${$app_layout}`,
                                    style: `guide`,
                                },
                                el: {
                                    classes: `font-sans text-[1.25rem] text-center placeholder:opacity-60`,
                                    id: fmt_id(`nostr_profilename`),
                                    sync: true,
                                    layer: 1,
                                    placeholder: `${$ls(`icu.enter_*`, { value: `${$ls(`common.profile_name`)}`.toLowerCase() })}`,
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
                            {`${$ls(`common.setup_for_farmer`)}`}
                        </p>
                    </div>
                    <div
                        class={`flex flex-col w-full gap-5 justify-center items-center`}
                    >
                        <button
                            class={`flex flex-col h-touch_bold w-${$app_layout} justify-center items-center rounded-touch ${cfg_main_opt_idx1 === `farmer` ? `layer-1-surface-apply-active layer-1-raise-apply layer-1-ring-apply` : `bg-layer-1-surface`} el-re`}
                            on:click|stopPropagation={async () => {
                                cfg_main_opt_idx1 = `farmer`;
                            }}
                        >
                            <p
                                class={`font-sans font-[600] text-layer-0-glyph text-xl`}
                            >
                                {`${$ls(`common.yes`)}`}
                            </p>
                        </button>
                        <button
                            class={`flex flex-col h-touch_bold w-${$app_layout} justify-center items-center rounded-touch ${cfg_main_opt_idx1 === `personal` ? `layer-1-surface-apply-active layer-1-raise-apply layer-1-ring-apply` : `bg-layer-1-surface`} el-re`}
                            on:click|stopPropagation={async () => {
                                cfg_main_opt_idx1 = `personal`;
                            }}
                        >
                            <p
                                class={`font-sans font-[600] text-layer-0-glyph text-xl`}
                            >
                                {`${$ls(`common.no`)}`}
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
                    {`${$ls(`icu.go_*`, { value: `${$ls(`common.back`)}` })}`}
                </p>
            </button>
        </div>
        <div
            class={`absolute max-mobile_base:bottom-0 bottom-8 left-0 flex flex-col w-full justify-center items-center ${view === `cfg_main` ? `fade-in-long` : ``}`}
        >
            <ButtonLayoutPair
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
                                ? `${$ls(`common.skip`)}`
                                : `${$ls(`common.back`)}`,
                        callback: async () => {
                            if ($carousel_index === 0) {
                                const confirm = await dialog.confirm({
                                    message: `${$ls(`app.cfg.init.notification.no_profile_name`)}`,
                                    cancel_label: `${$ls(`icu.add_*`, { value: `${$ls(`common.profile`)}` })}`,
                                    ok_label: `${$ls(`common.continue`)}`,
                                });
                                if (confirm === false) {
                                    el_id(fmt_id(`nostr_profilename`))?.focus();
                                    return;
                                }
                                carousel_inc(view);
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
                                {`${$ls(`eula.title`)}`}
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
                                    {`**${$ls(`eula.introduction.title`)}**`}
                                </p>
                                <p
                                    class={`font-mono font-[500] text-layer-0-glyph text-sm text-justify break-word`}
                                >
                                    {`${$ls(`eula.introduction.body`)}`}
                                </p>
                            </div>
                            <div
                                class={`flex flex-col w-full gap-2 justify-start items-start`}
                            >
                                <p
                                    class={`font-mono font-[600] text-layer-0-glyph`}
                                >
                                    {`**${$ls(`eula.prohibited_content.title`)}**`}
                                </p>
                                <p
                                    class={`font-mono font-[500] text-sm text-layer-0-glyph text-justify break-word`}
                                >
                                    {`${$ls(`eula.prohibited_content.body_0_title`)}`}
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
                                                    {`${$ls(`eula.prohibited_content.body_li_0_${li}`)}`}
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
                                    {`**${$ls(`eula.prohibited_conduct.title`)}**`}
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
                                                    {`${$ls(`eula.prohibited_conduct.body_li_0_${li}`)}`}
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
                                    {`**${$ls(`eula.consequences_of_violation.title`)}**`}
                                </p>
                                <p
                                    class={`font-mono font-[500] text-layer-0-glyph text-sm text-justify break-word`}
                                >
                                    {`${$ls(`eula.consequences_of_violation.body`)}`}
                                </p>
                            </div>
                            <div
                                class={`flex flex-col w-full gap-2 justify-start items-start`}
                            >
                                <p
                                    class={`font-mono font-[600] text-layer-0-glyph`}
                                >
                                    {`**${$ls(`eula.disclaimer.title`)}**`}
                                </p>
                                <p
                                    class={`font-mono font-[500] text-layer-0-glyph text-sm text-justify break-word`}
                                >
                                    {`${$ls(`eula.disclaimer.body`)}`}
                                </p>
                            </div>
                            <div
                                class={`flex flex-col w-full gap-2 justify-start items-start`}
                            >
                                <p
                                    class={`font-mono font-[600] text-layer-0-glyph`}
                                >
                                    {`**${$ls(`eula.changes.title`)}**`}
                                </p>
                                <p
                                    class={`font-mono font-[500] text-layer-0-glyph text-sm text-justify break-word`}
                                >
                                    {`${$ls(`eula.changes.body`)}`}
                                </p>
                            </div>
                            <div
                                class={`flex flex-col w-full gap-2 justify-start items-start`}
                            >
                                <p
                                    class={`font-mono font-[600] text-layer-0-glyph`}
                                >
                                    {`**${$ls(`eula.contact.title`)}**`}
                                </p>
                                <p
                                    class={`font-mono font-[500] text-layer-0-glyph text-sm text-justify break-word`}
                                >
                                    {`${$ls(`eula.contact.body`)}`}
                                </p>
                            </div>
                            <div
                                class={`flex flex-col w-full gap-2 justify-start items-start`}
                            >
                                <p
                                    class={`font-mono font-[600] text-layer-0-glyph`}
                                >
                                    {`**${$ls(`eula.acceptance_of_terms.title`)}**`}
                                </p>
                                <p
                                    class={`font-mono font-[500] text-layer-0-glyph text-sm text-justify break-word`}
                                >
                                    {`${$ls(`eula.acceptance_of_terms.body`)}`}
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
                                    message: `${$ls(`eula.error.required`)}`,
                                    cancel_label: `${$ls(`common.quit`)}`,
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
                                {`${`${$ls(`common.disagree`)}`}`}
                            </p>
                            <p
                                class={`font-mono font-[400] text-sm text-layer-0-glyph/60 group-active:text-layer-0-glyph transition-all`}
                            >
                                {`-`}
                            </p>
                        </button>
                        <button
                            class={`relative group flex flex-row basis-1/2 gap-4 justify-center items-center ${el_eula_scrolled ? `` : `opacity-40`}`}
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
                                {`${`${$ls(`common.agree`)}`}`}
                            </p>
                            <p
                                class={`font-mono font-[400] text-sm text-layer-0-glyph-hl group-active:text-layer-0-glyph-hl/80 transition-all`}
                            >
                                {`- `}
                            </p>
                            {#if loading_submit}
                                <div
                                    class={`absolute right-2 flex flex-row justify-start items-center`}
                                >
                                    <Loading basis={{ dim: `xs` }} />
                                </div>
                            {/if}
                        </button>
                    </div>
                </div>
            </div>
        </div>
    </div>
</LayoutView>
