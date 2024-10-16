<script lang="ts">
    import {
        PUBLIC_NOSTR_RELAY_DEFAULTS,
        PUBLIC_RADROOTS_URL,
    } from "$env/static/public";
    import { db, dialog, http, keystore, nostr } from "$lib/client";
    import ButtonAppearingPair from "$lib/components/button_appearing_pair.svelte";
    import { cfg, ks } from "$lib/conf";
    import { restart } from "$lib/utils/client";
    import {
        app_layout,
        carousel_index,
        carousel_index_max,
        carousel_next,
        carousel_prev,
        fmt_id,
        Glyph,
        InputElement,
        kv,
        LayoutView,
        Loading,
        sleep,
        t,
        view_effect,
    } from "@radroots/svelte-lib";
    import { regex } from "@radroots/utils";
    import { onDestroy, onMount } from "svelte";

    const carousel_param: Record<View, { max_index: number }> = {
        setup: {
            max_index: 2,
        },
        profile_name: {
            max_index: 2,
        },
        eula: {
            max_index: 1,
        },
    };

    let el_eula: HTMLDivElement;
    let el_eula_scrolled = false;

    type KeypairOption = `nostr_key_gen` | `nostr_key_existing`;
    let setup_keypair_option: KeypairOption | undefined = undefined;

    let profile_name_is_valid = false;
    let profile_name_loading = false;

    type View = `setup` | `profile_name` | `eula`;
    let view: View = `setup`;

    $: {
        view_effect<View>(view);
    }

    onMount(async () => {
        try {
            handle_view(`setup`);

            await keystore.remove(ks.nostr.conf_init_key);
            await keystore.remove(ks.nostr.conf_init_profile);

            el_eula.addEventListener("scroll", () => {
                const client_h = el_eula.clientHeight;
                const scroll_h = el_eula.scrollHeight;
                const scroll_top = el_eula.scrollTop;
                if (scroll_top + client_h >= scroll_h) el_eula_scrolled = true;
            });
        } catch (e) {
        } finally {
        }
    });

    onDestroy(async () => {
        try {
            el_eula.removeEventListener("scroll", () => null);
        } catch (e) {
        } finally {
        }
    });

    const handle_view = (new_view: View): void => {
        if (new_view === `setup` && view === `profile_name`) {
            const offset = setup_keypair_option === `nostr_key_gen` ? 1 : 0;
            carousel_index.set(carousel_param[new_view].max_index - offset);
        } else {
            carousel_index.set(0);
        }
        carousel_index_max.set(carousel_param[new_view].max_index);
        view = new_view;
    };

    const handle_back = async (): Promise<void> => {
        try {
            switch (view) {
                case `setup`:
                    {
                        switch ($carousel_index) {
                            case 1:
                                {
                                    setup_keypair_option = undefined;
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
                case `profile_name`: {
                    switch ($carousel_index) {
                        case 0:
                            {
                                handle_view(`setup`);
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
                case `setup`:
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
                                        setup_keypair_option === `nostr_key_gen`
                                    ) {
                                        const nostr_sk_previous =
                                            await keystore.get(
                                                ks.nostr.conf_init_key,
                                            );
                                        if (`result` in nostr_sk_previous) {
                                            handle_view(`profile_name`);
                                            return;
                                        }
                                        const nostr_sk =
                                            nostr.lib.generate_key();
                                        const ks_set = await keystore.set(
                                            ks.nostr.conf_init_key,
                                            nostr_sk,
                                        );
                                        if (`err` in ks_set) {
                                            await dialog.alert(
                                                `${$t(`error.client.unhandled`)}`,
                                            );
                                            return; //@todo
                                        }
                                        handle_view(`profile_name`);
                                    } else if (
                                        setup_keypair_option ===
                                        `nostr_key_existing`
                                    )
                                        await carousel_next(view);
                                }
                                break;
                            case 2:
                                {
                                    const nostr_key_existing = await kv.get(
                                        fmt_id(`setup_nostr_key_existing`),
                                    );
                                    if (!nostr_key_existing) {
                                        await dialog.alert(
                                            `${$t(`icu.enter_a_*`, { value: `${$t(`icu.valid_*`, { value: `${$t(`common.key`)}` })}`.toLowerCase() })}`,
                                        );
                                        return;
                                    }
                                    const valid_nostr_key_nsec =
                                        nostr.lib.nsec_decode(
                                            nostr_key_existing,
                                        );
                                    if (valid_nostr_key_nsec) {
                                        const ks_set = await keystore.set(
                                            ks.nostr.conf_init_key,
                                            valid_nostr_key_nsec,
                                        );
                                        if (`err` in ks_set) {
                                            await dialog.alert(
                                                `${$t(`error.client.unhandled`)}`,
                                            );
                                            return; //@todo
                                        }
                                        handle_view(`eula`);
                                    }
                                    const valid_nostr_key_hex =
                                        nostr.lib.public_key(
                                            nostr_key_existing,
                                        );
                                    if (valid_nostr_key_hex) {
                                        const ks_set = await keystore.set(
                                            ks.nostr.conf_init_key,
                                            nostr_key_existing,
                                        );
                                        if (`err` in ks_set) {
                                            await dialog.alert(
                                                `${$t(`error.client.unhandled`)}`,
                                            );
                                            return; //@todo
                                        }
                                        handle_view(`eula`);
                                    }
                                    await dialog.alert(
                                        `${$t(`icu.invalid_*`, { value: `${$t(`common.key`)}`.toLowerCase() })}`,
                                    );
                                }
                                break;
                        }
                    }
                    break;
                case `profile_name`:
                    {
                        switch ($carousel_index) {
                            case 0:
                                {
                                    if (profile_name_loading) return;
                                    const profile_name = await kv.get(
                                        fmt_id(`profile_name`),
                                    );
                                    if (!profile_name) {
                                        await dialog.alert(
                                            `${$t(`icu.enter_a_*`, { value: `${$t(`common.profile_name`)}`.toLowerCase() })}`,
                                        );
                                        return;
                                    }
                                    const valid_profile_name =
                                        await fetch_validate_profile_name(
                                            profile_name,
                                        );
                                    if (
                                        typeof valid_profile_name === `string`
                                    ) {
                                        await dialog.alert(valid_profile_name);
                                        return;
                                    } else if (!valid_profile_name) {
                                        await dialog.alert(
                                            `${`${$t(`icu.the_*_is_registered`, { value: `${$t(`common.profile_name`)}` })}`}`,
                                        );
                                        return;
                                    }

                                    const confirm = await dialog.confirm({
                                        message: `${`${$t(`icu.the_*_is_available`, { value: `${$t(`common.profile_name`).toLowerCase()} "${profile_name}"` })}`}. Would you like to use it?`,
                                        cancel_label: `${$t(`common.no`)}`,
                                        ok_label: `${$t(`common.yes`)}`,
                                    });
                                    if (confirm === true) {
                                        const ks_set = await keystore.set(
                                            ks.nostr.conf_init_profile,
                                            profile_name,
                                        );
                                        if (`err` in ks_set) {
                                            await dialog.alert(
                                                `${$t(`error.client.unhandled`)}`,
                                            );
                                            return; //@todo
                                        }
                                        handle_view(`eula`);
                                    }
                                }
                                break;
                        }
                    }
                    break;
            }
        } catch (e) {
            console.log(`(error) handle_continue `, e);
        }
    };

    const fetch_validate_profile_name = async (
        profile_name: string,
    ): Promise<boolean | string> => {
        try {
            profile_name_loading = true;
            const res = await http.fetch({
                url: `${PUBLIC_RADROOTS_URL}/.well-known/nostr.json`,
            });
            if (`err` in res) {
                return res.err;
            }

            const { data: res_data } = res;
            if (`names` in res_data) {
                if (typeof res_data.names[profile_name] === `undefined`)
                    return true;
                return false;
            }

            return `error`;
        } catch (e) {
            console.log(`(error) fetch_validate_profile_name `, e);
            return `catch`;
        } finally {
            profile_name_loading = false;
        }
    };

    const configure_device = async (): Promise<void> => {
        try {
            const conf_init_secret_key = await keystore.get(
                ks.nostr.conf_init_key,
            );
            if (`err` in conf_init_secret_key) {
                alert(`!conf_init_secret_key`);
                return; //@todo
            }

            const secret_key = conf_init_secret_key.result;
            const public_key = nostr.lib.public_key(secret_key);
            const ks_key_add = await keystore.set(
                ks.nostr.nostr_key(public_key),
                secret_key,
            );
            if (!ks_key_add) {
                alert(`!ks_key_add`);
                return; //@todo
            }

            const pref_key_add = await keystore.set(
                ks.nostr.nostr_key_active,
                public_key,
            );
            if (!pref_key_add) {
                alert(`!pref_key_add`);
                return; //@todo
            }

            let profile_name = ``;
            const conf_init_profile = await keystore.get(
                ks.nostr.conf_init_profile,
            );
            if (`result` in conf_init_profile) {
                profile_name = conf_init_profile.result;
            }

            const nostr_profile_add = await db.nostr_profile_add({
                public_key,
                name: profile_name ? profile_name : undefined,
            });

            if (`err` in nostr_profile_add) {
                await dialog.alert(nostr_profile_add.err);
                return; //@todo
            } else if (`err_s` in nostr_profile_add) {
                await dialog.alert(nostr_profile_add.err_s.join(` `));
                return; //@todo
            }

            for (const url of PUBLIC_NOSTR_RELAY_DEFAULTS.split(",") || []) {
                const nostr_relay_add = await db.nostr_relay_add({ url });
                if (`err` in nostr_relay_add) {
                    await dialog.alert(nostr_relay_add.err);
                    return; //@todo
                } else if (`err_s` in nostr_relay_add) {
                    return; //@todo
                }
                await db.set_nostr_profile_relay({
                    nostr_profile: {
                        id: nostr_profile_add.id,
                    },
                    nostr_relay: {
                        id: nostr_relay_add.id,
                    },
                });
            }

            await keystore.remove(ks.nostr.conf_init_key);
            await keystore.remove(ks.nostr.conf_init_profile);

            await sleep(cfg.delay.load);
            await restart(
                true,
                `${$t(`app.page.conf.init.notification.welcome`)}`,
            );
        } catch (e) {
            console.log(`(error) configure_device `, e);
        }
    };
</script>

<LayoutView>
    <div
        data-view={`setup`}
        class={`flex flex-col h-full w-full max-mobile_base:pt-12 pt-16 justify-start items-center`}
    >
        <div
            data-carousel-container={`setup`}
            class={`carousel-container flex h-full w-full`}
        >
            <div
                data-carousel-item={`setup`}
                class={`carousel-item flex flex-col w-full max-mobile_y:pt-28 pt-32 pb-4 justify-start items-center`}
            >
                <div class={`flex flex-col gap-8 justify-start items-center`}>
                    <div
                        class={`flex flex-col gap-1 justify-start items-center`}
                    >
                        <p
                            class={`font-mono font-[700] text-layer-0-glyph text-4xl`}
                        >
                            {`${`${$t(`app.name`)}`}`}
                        </p>
                        <p
                            class={`font-mono font-[700] text-layer-0-glyph text-4xl`}
                        >
                            {`${`${$t(`common.setup`)}`}`}
                        </p>
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
                data-carousel-item={`setup`}
                class={`carousel-item flex flex-col w-full max-mobile_y:pt-32 pt-36 pb-4 justify-start items-center`}
                on:click={async () => {
                    setup_keypair_option = undefined;
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
                            class={`flex flex-col h-touch_bold w-${$app_layout} justify-center items-center bg-layer-1-surface rounded-3xl touch-layer-1 touch-layer-1-raise-less ${setup_keypair_option === `nostr_key_gen` ? `layer-1-ring` : ``}`}
                            on:click|stopPropagation={async () => {
                                setup_keypair_option = `nostr_key_gen`;
                            }}
                        >
                            <p
                                class={`font-sans font-[600] text-layer-0-glyph text-xl`}
                            >
                                {`${$t(`icu.create_new_*`, { value: `${$t(`common.keypair`)}`.toLowerCase() })}`}
                            </p>
                        </button>
                        <button
                            class={`flex flex-col h-touch_bold w-${$app_layout} justify-center items-center bg-layer-1-surface rounded-3xl touch-layer-1 touch-layer-1-raise-less ${setup_keypair_option === `nostr_key_existing` ? `layer-1-ring` : ``}`}
                            on:click|stopPropagation={async () => {
                                setup_keypair_option = `nostr_key_existing`;
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
                data-carousel-item={`setup`}
                class={`carousel-item flex flex-col w-full max-mobile_y:pt-32 pt-36 pb-4 justify-start items-center`}
            >
                <div
                    class={`flex flex-col w-full gap-8 justify-start items-center ${view === `profile_name` ? `fade-in-long` : ``}`}
                >
                    <div
                        class={`flex flex-col w-full gap-6 justify-center items-center`}
                    >
                        <p
                            class={`font-mono font-[600] text-layer-0-glyph text-3xl`}
                        >
                            {`${$t(`icu.add_existing_*`, { value: `${$t(`common.key`)}`.toLowerCase() })}`}
                        </p>
                        <InputElement
                            basis={{
                                classes: `h-entry_guide w-${$app_layout} bg-layer-1-surface rounded-touch font-mono text-lg placeholder:opacity-60 items-end text-center`,
                                id: fmt_id(`setup_nostr_key_existing`),
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
                    </div>
                </div>
            </div>
        </div>
        <div
            class={`absolute max-mobile_base:bottom-0 bottom-8 left-0 flex flex-col w-full justify-center items-center ${view === `profile_name` ? `fade-in-long` : ``}`}
        >
            <ButtonAppearingPair
                basis={{
                    continue: {
                        disabled:
                            $carousel_index === 1 && !setup_keypair_option,
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
        data-view={`profile_name`}
        class={`hidden flex flex-col h-full w-full max-mobile_base:pt-12 pt-16 justify-start items-center`}
    >
        <div
            data-carousel-container={`profile_name`}
            class={`carousel-container flex h-full w-full`}
        >
            <div
                data-carousel-item={`profile_name`}
                class={`carousel-item flex flex-col w-full max-mobile_y:pt-32 pt-36 pb-4 justify-start items-center`}
            >
                <div
                    class={`flex flex-col w-full gap-4 justify-start items-center ${view === `profile_name` ? `fade-in-long` : ``}`}
                >
                    <div
                        class={`flex flex-col w-full gap-2 justify-center items-center`}
                    >
                        <p
                            class={`font-mono font-[600] text-layer-0-glyph text-3xl capitalize`}
                        >
                            {`${$t(`icu.add_*`, { value: `${$t(`common.profile`)}` })}`}
                        </p>
                    </div>
                    <div
                        class={`flex flex-col h-[6.7rem] w-${$app_layout} pt-6 pb-6 px-8 justify-start items-center bg-layer-1-surface rounded-3xl`}
                    >
                        <div
                            class={`relative flex flex-row w-full justify-center items-center border-b-[2px] border-b-layer-1-surface-edge`}
                        >
                            <InputElement
                                basis={{
                                    classes: `font-mono text-[1.05rem] placeholder:text-layer-1-glyph/40 tracking-wider items-end text-center`,
                                    id: fmt_id(`profile_name`),
                                    sync: true,
                                    sync_init: true,
                                    placeholder: `${$t(`icu.name_your_*`, { value: `${$t(`common.profile`)}` })}`,
                                    field: {
                                        charset: regex.profile_name_ch,
                                        validate: regex.profile_name,
                                        validate_keypress: true,
                                    },
                                    callback: async ({ pass }) => {
                                        profile_name_is_valid = pass;
                                    },
                                    callback_keydown: async ({ key, el }) => {
                                        if (key === `Enter`) {
                                            el.blur();
                                            await handle_continue();
                                        }
                                    },
                                }}
                            />
                            {#if profile_name_loading}
                                <div
                                    class={`absolute top-0 right-0 flex flex-row h-full pr-2 justify-center items-center`}
                                >
                                    <Loading />
                                </div>
                            {/if}
                        </div>
                    </div>
                </div>
            </div>
        </div>
        <div
            class={`absolute top-16 left-4 flex flex-row gap-2 justify-start items-center ${view === `profile_name` ? `fade-in-long` : ``}`}
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
            class={`absolute max-mobile_base:bottom-0 bottom-8 left-0 flex flex-col w-full justify-center items-center ${view === `profile_name` ? `fade-in-long` : ``}`}
        >
            <ButtonAppearingPair
                basis={{
                    continue: {
                        disabled: !profile_name_is_valid,
                        callback: async () => await handle_continue(),
                    },
                    back: {
                        visible: true,
                        label: `${$t(`common.skip`)}`,
                        callback: async () => {
                            //@todo
                            await dialog.alert(
                                `${$t(`app.page.conf.init.notification.no_profile_name`)}`,
                            );
                            handle_view(`eula`);
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
                    class={`flex flex-col w-full px-4 pb-8 justify-start items-center ${view === `eula` ? `fade-in-long` : ``}`}
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
                        class={`flex flex-row h-20 w-full justify-center items-center`}
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
                                if (el_eula_scrolled) await configure_device();
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
