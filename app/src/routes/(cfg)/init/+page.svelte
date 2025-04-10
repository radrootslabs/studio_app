<script lang="ts">
    import { goto } from "$app/navigation";
    import { PUBLIC_NOSTR_RELAY_DEFAULTS } from "$env/static/public";
    import { ls } from "$lib/locale/i18n";
    import { datastore, db, gui, keys, radroots, route } from "$lib/util";
    import { cfg_delay } from "$lib/util/conf";
    import { NDKPrivateKeySigner } from "@nostr-dev-kit/ndk";
    import {
        app_lo,
        app_loading,
        ButtonLayoutPair,
        carousel_dec,
        carousel_inc,
        carousel_index,
        carousel_index_max,
        EntryLine,
        fmt_id,
        handle_err,
        LoadSymbol,
        LogoCircle,
        view_effect,
    } from "@radroots/lib-app";
    import { el_id, form_fields, sleep, type ResultPass } from "@radroots/util";
    import { onMount } from "svelte";

    const page_carousel: Record<View, { max_index: number }> = {
        cfg_key: {
            max_index: 2,
        },
        cfg_profile: {
            max_index: 2,
        },
        eula: {
            max_index: 1,
        },
    };

    let el_eula_scrolled = $state(false);
    type View = `cfg_key` | `cfg_profile` | `eula`;
    let initial_view: View = `cfg_key`;
    let view: View = $state(initial_view);

    $effect(() => {
        view_effect<View>(view);
    });

    let loading_submit = $state(false);

    type CfgRole = `farmer` | `personal`;
    let cfg_role: CfgRole | undefined = $state(undefined);

    type CfgKeyOpt = `nostr_keygen` | `nostr_keyadd`;
    let cgf_key_opt: CfgKeyOpt | undefined = $state(undefined);
    let cfg_key_add_existing_val = $state(``);

    let cfg_profile_nip05_opt = $state(false);
    let cfg_profile_name_val = $state(``);
    let cfg_profile_name_valid = $state(false);
    let cfg_profile_name_loading = $state(false);

    const cfg_profile_name_skip = $derived(
        view.toString() === `cfg_profile` && $carousel_index === 0,
    );

    onMount(async () => {
        try {
            await init();
        } catch (e) {
            handle_err(e, `on_mount`);
        }
    });

    const init = async (): Promise<void> => {
        const nostrkey_all = await keys.nostr_read_all(); //@todo
        handle_view(view);
    };

    const handle_view = (new_view: View): void => {
        if (new_view === `cfg_key` && view === `cfg_profile`) {
            const offset = cgf_key_opt === `nostr_keygen` ? 1 : 0;
            carousel_index.set(page_carousel[new_view].max_index - offset);
        } else {
            carousel_index.set(0);
            carousel_index_max.set(page_carousel[new_view].max_index);
        }
        view = new_view;
    };

    const on_scroll_eula = async ({
        currentTarget: el,
    }: {
        currentTarget: EventTarget & HTMLDivElement;
    }): Promise<void> => {
        const client_h = el?.clientHeight;
        const scroll_h = el?.scrollHeight;
        const scroll_top = el?.scrollTop;
        if (scroll_top + client_h >= scroll_h) el_eula_scrolled = true;
    };

    const reset = async (
        alert_message?: string,
        prevent_loading?: boolean,
    ): Promise<void> => {
        try {
            app_loading.set(!prevent_loading);
            handle_view(`cfg_key`);
            if (alert_message) await gui.alert(alert_message);
            await sleep(cfg_delay.load);
            await keys.nostr_keystore_reset();
            await db.reset();
        } catch (e) {
            await handle_err(e, `reset`);
        } finally {
            app_loading.set(false);
        }
    };

    const key_add_keystore = async (public_key: string): Promise<void> => {
        const ks_key_nostr_add = await datastore.set(`key_nostr`, public_key);
        if (`err` in ks_key_nostr_add)
            return void (await reset(
                `${$ls(`error.init.configuration_failure`)}`,
            ));
    };

    const key_gen = async (): Promise<void> => {
        const keys_nostr_create = await keys.nostr_gen();
        if (`err` in keys_nostr_create)
            return void (await reset(
                `${$ls(`error.init.configuration_failure`)}`,
            ));
        await datastore.set(`init_nostr`, keys_nostr_create.public_key);
    };

    const key_add = async (secret_key: string): Promise<void> => {
        const keys_nostr_add = await keys.nostr_add(secret_key);
        console.log(`keys_nostr_add `, keys_nostr_add);
        if (`err` in keys_nostr_add)
            return void (await gui.alert(`${$ls(`common.invalid_key`)}`));
        await datastore.set(`init_nostr`, keys_nostr_add.public_key);
    };

    const configure_device = async (
        public_key: string,
        profile_name?: string,
    ): Promise<ResultPass | void> => {
        const nostr_profile_add = await db.nostr_profile_create({
            public_key,
            name: profile_name ? profile_name : undefined,
            display_name: profile_name ? profile_name : undefined,
        });
        if (`err` in nostr_profile_add || `err_s` in nostr_profile_add)
            return void (await gui.alert(
                `${$ls(`error.init.device_configuration_nostr_profile`)}`,
            ));
        for (const url of Array.from(
            new Set([...PUBLIC_NOSTR_RELAY_DEFAULTS.split(",")]),
        )) {
            const nostr_relay_add = await db.nostr_relay_create({ url });
            if (`err` in nostr_relay_add || `err_s` in nostr_relay_add)
                return void (await gui.alert(
                    `${$ls(`error.init.device_configuration_nostr_relay`)}`,
                ));
            await db.nostr_profile_relay_set({
                nostr_profile: {
                    id: nostr_profile_add.id,
                },
                nostr_relay: {
                    id: nostr_relay_add.id,
                },
            });
        }
        await key_add_keystore(public_key);
        return { pass: true };
    };

    const confirm_profile_add_without_name = async (): Promise<boolean> => {
        const confirm = await gui.confirm({
            message: `${$ls(`notification.init.no_profile_option`)}`,
            cancel: `${$ls(`icu.add_*`, { value: `${$ls(`common.profile`)}` })}`,
            ok: `${$ls(`common.continue`)}`,
        });
        return confirm;
    };

    const handle_choose_key_gen_or_add = async (): Promise<void> => {
        try {
            if (cgf_key_opt === `nostr_keyadd`)
                return void (await carousel_inc(view));
            await key_gen();
            handle_view(`cfg_profile`);
        } catch (e) {
            await handle_err(e, `handle_choose_key_gen_or_add`);
        }
    };

    const handle_key_add_existing = async (): Promise<void> => {
        try {
            if (!cfg_key_add_existing_val)
                return void (await gui.alert(
                    `${$ls(`icu.enter_a_*`, { value: `${$ls(`common.nostr_key`)}`.toLowerCase() })}`,
                ));
            const key_add_signer = new NDKPrivateKeySigner(
                cfg_key_add_existing_val,
            );
            const key_add_user = await key_add_signer.user();
            const key_add_hex_secret_key = key_add_signer.privateKey;
            const key_add_hex_public_key = key_add_user.pubkey;
            const key_nostr_read = await keys.nostr_read(
                cfg_key_add_existing_val,
            );
            if (
                (`err` in key_nostr_read &&
                    key_nostr_read.err !== `error.keystore.key_not_found`) ||
                !key_add_hex_secret_key ||
                !key_add_hex_public_key
            )
                return void (await gui.alert(
                    `${$ls(`icu.enter_a_valid_*`, { value: `${$ls(`common.nostr_key`)}`.toLowerCase() })}`,
                ));
            await key_add(key_add_hex_secret_key);
            cfg_key_add_existing_val = ``;
            return void handle_view(`cfg_profile`);
        } catch (e) {
            await handle_err(e, `handle_key_add_existing`);
            return void (await gui.alert(
                `${$ls(`icu.not_a_valid_*`, { value: `${$ls(`common.nostr_key`)}`.toLowerCase() })}`,
            ));
        }
    };

    const handle_profile_add = async (): Promise<void> => {
        try {
            if (cfg_profile_name_loading) return;
            const ds_nostr_key_init = await datastore.get(`init_nostr`);
            if (`err` in ds_nostr_key_init)
                return void (await reset(
                    `${$ls(`error.init.configuration_failure`)}`,
                )); //@todo
            const key_nostr_read = await keys.nostr_read(
                ds_nostr_key_init.result,
            );
            if (`err` in key_nostr_read)
                return void (await reset(
                    `${$ls(`error.init.configuration_failure`)}`,
                )); //@todo
            if (cfg_profile_nip05_opt) {
                if (!cfg_profile_name_val)
                    return void (await gui.alert(
                        `${$ls(`icu.enter_a_*`, { value: `${$ls(`common.profile_name`)}`.toLowerCase() })}`,
                    ));
                cfg_profile_name_loading = true;
                const profile_req = await radroots.fetch_profile_request({
                    profile_name: cfg_profile_name_val,
                    secret_key: key_nostr_read.secret_key,
                });
                if (`err` in profile_req)
                    return void (await gui.alert(
                        `${$ls(profile_req.err, { default: `${$ls(`error.client.http.request_failure`)}` })}`,
                    ));
                const confirm = await gui.confirm({
                    message: `${`${$ls(`icu.the_*_is_available`, { value: `${$ls(`common.profile_name`).toLowerCase()} "${cfg_profile_name_val}"` })}`}. ${`${$ls(`common.would_you_like_to_use_it_q`)}`}`,
                    cancel: `${$ls(`common.no`)}`,
                    ok: `${$ls(`common.yes`)}`,
                });
                if (!confirm) return;
                const profile_create = await radroots.fetch_profile_create({
                    tok: profile_req.result,
                    secret_key: key_nostr_read.secret_key,
                });
                if (`err` in profile_create)
                    return void (await gui.alert(
                        `${$ls(profile_create.err, { default: `${$ls(`error.client.http.request_failure`)}` })}`,
                    ));
                await datastore.setp(
                    `radroots_profile`,
                    ds_nostr_key_init.result,
                    profile_create.result,
                );
            } else if (!cfg_profile_name_val) {
                const confirm = await confirm_profile_add_without_name();
                if (!confirm)
                    return void el_id(fmt_id(`nostr:profile`))?.focus();
            }
            await carousel_inc(view);
        } catch (e) {
            await handle_err(e, `handle_profile_add`);
        } finally {
            cfg_profile_name_loading = false;
        }
    };

    const handle_set_role = async (): Promise<void> => {
        if (!cfg_role) cfg_role = `personal`;
        await datastore.set(`role`, cfg_role);
        await datastore.set(`is_setup`, new Date().toISOString());
        handle_view(`eula`);
    };

    const handle_continue = async (): Promise<void> => {
        switch (view) {
            case `cfg_key`:
                switch ($carousel_index) {
                    case 0:
                        return await carousel_inc(view);
                    case 1:
                        return await handle_choose_key_gen_or_add();
                    case 2:
                        return await handle_key_add_existing();
                }
            case `cfg_profile`:
                switch ($carousel_index) {
                    case 0:
                        return await handle_profile_add();
                    case 1:
                        return await handle_set_role();
                }
        }
    };

    const handle_back = async (): Promise<void> => {
        switch (view) {
            case `cfg_key`:
                switch ($carousel_index) {
                    case 1: {
                        cgf_key_opt = undefined;
                        return await carousel_dec(view);
                    }
                    case 2: {
                        cfg_key_add_existing_val = ``;
                        return await carousel_dec(view);
                    }
                }
            case `cfg_profile`:
                switch ($carousel_index) {
                    case 0: {
                        if (cfg_profile_name_skip) {
                            const confirm =
                                await confirm_profile_add_without_name();
                            if (!confirm)
                                return void el_id(
                                    fmt_id(`nostr:profile`),
                                )?.focus();
                            return void carousel_inc(view);
                        }
                        return handle_view(`cfg_key`);
                    }
                    case 1:
                        return carousel_dec(view);
                }
        }
    };

    const submit = async (): Promise<void> => {
        try {
            loading_submit = true;
            const ds_nostr_key_init = await datastore.get(`init_nostr`);
            if (`err` in ds_nostr_key_init)
                return void (await reset(
                    `${$ls(`error.init.configuration_failure`)}`,
                )); //@todo
            const key_nostr_read = await keys.nostr_read(
                ds_nostr_key_init.result,
            );
            if (`err` in key_nostr_read)
                return void (await reset(
                    `${$ls(`error.init.configuration_failure`)}`,
                )); //@todo
            const radroots_profile = await datastore.getp(
                `radroots_profile`,
                ds_nostr_key_init.result,
            );
            if (`result` in radroots_profile) {
                await radroots.fetch_profile_activate({
                    id: radroots_profile.result,
                    secret_key: key_nostr_read.secret_key,
                }); //@todo
            }
            const configuration_result = await configure_device(
                ds_nostr_key_init.result,
                cfg_profile_name_val,
            );
            if (!configuration_result)
                return void (await reset(
                    `${$ls(`error.init.configuration_failure`)}`,
                )); //@todo
            route(`/`);
            await gui.notify_init();
        } catch (e) {
            await handle_err(e, `submit`);
        } finally {
            loading_submit = false;
        }
    };
</script>

<div
    data-view={`cfg_key`}
    class={`flex flex-col h-full w-full justify-start items-center`}
>
    <div
        data-carousel-container={`cfg_key`}
        class={`carousel-container flex h-full w-full`}
    >
        <div
            data-carousel-item={`cfg_key`}
            class={`carousel-item flex flex-col h-full w-full justify-center items-center`}
        >
            <div
                class={`relative flex flex-col h-full w-full justify-center items-center`}
            >
                <div
                    class={`flex flex-row w-full justify-start items-center -translate-y-16`}
                >
                    <button
                        class={`flex flex-row w-full justify-center items-center`}
                        onclick={async () => {
                            await goto(`/`);
                        }}
                    >
                        <LogoCircle />
                    </button>
                </div>
                <div
                    class={`absolute bottom-0 left-0 flex flex-col h-[20rem] w-full px-10 gap-2 justify-start items-center`}
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
                                {`${$ls(`notification.init.greeting_header`)}`}
                            </p>
                        </div>
                        <div
                            class={`flex flex-row w-full justify-start items-center`}
                        >
                            <p
                                class={`font-mono font-[400] text-[1.1rem] text-layer-0-glyph`}
                            >
                                {`${$ls(`notification.init.greeting_subheader`)}.`}
                            </p>
                        </div>
                    </div>
                </div>
            </div>
        </div>
        <div
            data-carousel-item={`cfg_key`}
            class={`carousel-item flex flex-col h-full w-full justify-center items-center`}
            role="button"
            tabindex="0"
            onclick={async () => {
                cgf_key_opt = undefined;
            }}
            onkeydown={null}
        >
            <div
                class={`flex flex-col h-[16rem] gap-8 w-full justify-start items-center`}
            >
                <div class={`flex flex-row w-full justify-center items-center`}>
                    <p
                        class={`font-sans font-[600] text-layer-0-glyph text-3xl`}
                    >
                        {`${$ls(`icu.configure_*`, { value: `${$ls(`common.device`)}` })}`}
                    </p>
                </div>
                <div
                    class={`flex flex-col w-full gap-6 justify-center items-center`}
                >
                    <button
                        class={`flex flex-col h-bold_button w-lo_${$app_lo} justify-center items-center rounded-touch ${cgf_key_opt === `nostr_keygen` ? `layer-1-surface-apply-active layer-1-raise-apply layer-1-ring-apply` : `bg-layer-1-surface`} el-re`}
                        onclick={async (ev) => {
                            ev.stopPropagation();
                            cgf_key_opt = `nostr_keygen`;
                        }}
                    >
                        <p
                            class={`font-sans font-[600] text-layer-0-glyph text-xl`}
                        >
                            {`${$ls(`icu.create_new_*`, { value: `${$ls(`common.keypair`)}`.toLowerCase() })}`}
                        </p>
                    </button>
                    <button
                        class={`flex flex-col h-bold_button w-lo_${$app_lo} justify-center items-center rounded-touch ${cgf_key_opt === `nostr_keyadd` ? `layer-1-surface-apply-active layer-1-raise-apply layer-1-ring-apply` : `bg-layer-1-surface`} el-re`}
                        onclick={async (ev) => {
                            ev.stopPropagation();
                            cgf_key_opt = `nostr_keyadd`;
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
        </div>
        <div
            data-carousel-item={`cfg_key`}
            class={`carousel-item flex flex-col h-full w-full justify-center items-center`}
        >
            <div
                class={`flex flex-col w-full gap-8 justify-start items-center`}
            >
                <div
                    class={`flex flex-col w-full gap-6 justify-center items-center`}
                >
                    <p
                        class={`font-sans font-[600] text-layer-0-glyph text-3xl capitalize`}
                    >
                        {`${$ls(`icu.add_existing_*`, { value: `${$ls(`common.key`)}`.toLowerCase() })}`}
                    </p>
                    <EntryLine
                        bind:value={cfg_key_add_existing_val}
                        basis={{
                            wrap: {
                                layer: 1,
                                classes: `w-lo_${$app_lo}`,
                                style: `guide`,
                            },
                            el: {
                                classes: `font-sans text-[1.25rem] text-center placeholder:opacity-60`,
                                layer: 1,
                                placeholder: `${$ls(`icu.enter_*`, { value: `nostr nsec/hex` })}`,
                                callback_keydown: async ({ key_s, el }) => {
                                    if (key_s) {
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
        <div
            class={`z-10 absolute max-m_0:bottom-0 bottom-10 left-0 flex flex-col w-full justify-center items-center`}
        >
            <ButtonLayoutPair
                basis={{
                    continue: {
                        label: `${$ls(`common.continue`)}`,
                        disabled: $carousel_index === 1 && !cgf_key_opt,
                        callback: async () => await handle_continue(),
                    },
                    back: {
                        label: `${$ls(`common.back`)}`,
                        visible: $carousel_index > 0,
                        callback: async () => await handle_back(),
                    },
                }}
            />
        </div>
    </div>
</div>
<div
    data-view={`cfg_profile`}
    class={`hidden flex flex-col h-full w-full justify-start items-center`}
>
    <div
        data-carousel-container={`cfg_profile`}
        class={`carousel-container flex h-full w-full`}
    >
        <div
            data-carousel-item={`cfg_profile`}
            class={`carousel-item flex flex-col h-full w-full justify-center items-center`}
        >
            <div
                class={`flex flex-col h-[16rem] w-full px-4 gap-6 justify-start items-center`}
            >
                <p class={`font-sans font-[600] text-layer-0-glyph text-3xl`}>
                    {`${$ls(`icu.add_*`, { value: `${$ls(`common.profile`)}` })}`}
                </p>
                <div
                    class={`flex flex-col w-full gap-4 justify-center items-center`}
                >
                    <EntryLine
                        bind:value={cfg_profile_name_val}
                        basis={{
                            loading: cfg_profile_name_loading,
                            wrap: {
                                layer: 1,
                                classes: `w-lo_${$app_lo}`,
                                style: `guide`,
                            },
                            el: {
                                classes: `font-sans text-[1.25rem] text-center placeholder:opacity-60`,
                                id: fmt_id(`nostr:profile`),
                                layer: 1,
                                placeholder: `${$ls(`icu.enter_*`, { value: `${$ls(`common.profile_name`)}`.toLowerCase() })}`,
                                field: form_fields.profile_name,
                                callback: async ({ pass }) => {
                                    cfg_profile_name_valid = pass;
                                },
                                callback_keydown: async ({ key_s, el }) => {
                                    if (key_s) {
                                        el.blur();
                                        await handle_continue();
                                    }
                                },
                            },
                        }}
                    />
                    <div
                        class={`flex flex-row w-full gap-2 justify-center items-center`}
                    >
                        <input
                            type="checkbox"
                            bind:checked={cfg_profile_nip05_opt}
                        />
                        <button
                            class={`flex flex-row justify-center items-center`}
                            onclick={async () => {
                                cfg_profile_nip05_opt = !cfg_profile_nip05_opt;
                            }}
                        >
                            <p
                                class={`font-sans font-[400] text-layer-0-glyph text-[14px] tracking-wide`}
                            >
                                {`${$ls(`common.create`)}`}
                                <span
                                    class={`font-mono font-[600] tracking-tight px-[3px]`}
                                >
                                    {`@radroots`}
                                </span>
                                {`${$ls(`common.nip05_address`)}`}
                            </p>
                        </button>
                    </div>
                </div>
            </div>
        </div>
        <div
            data-carousel-item={`cfg_profile`}
            class={`carousel-item flex flex-col h-full w-full justify-center items-center`}
            role="button"
            tabindex="0"
            onclick={async () => {
                cfg_role = undefined;
            }}
            onkeydown={null}
        >
            <div
                class={`flex flex-col h-[16rem] w-full gap-10 justify-start items-center`}
            >
                <div class={`flex flex-row w-full justify-center items-center`}>
                    <p
                        class={`font-sans font-[600] text-layer-0-glyph text-3xl`}
                    >
                        {`${$ls(`common.setup_for_farmer`)}`}
                    </p>
                </div>
                <div
                    class={`flex flex-col w-full gap-5 justify-center items-center`}
                >
                    <button
                        class={`flex flex-col h-bold_button w-lo_${$app_lo} justify-center items-center rounded-touch ${cfg_role === `farmer` ? `layer-1-surface-apply-active layer-1-raise-apply layer-1-ring-apply` : `bg-layer-1-surface`} el-re`}
                        onclick={async (ev) => {
                            ev.stopPropagation();
                            cfg_role = `farmer`;
                        }}
                    >
                        <p
                            class={`font-sans font-[600] text-layer-0-glyph text-xl`}
                        >
                            {`${$ls(`common.yes`)}`}
                        </p>
                    </button>
                    <button
                        class={`flex flex-col h-bold_button w-lo_${$app_lo} justify-center items-center rounded-touch ${cfg_role === `personal` ? `layer-1-surface-apply-active layer-1-raise-apply layer-1-ring-apply` : `bg-layer-1-surface`} el-re`}
                        onclick={async (ev) => {
                            ev.stopPropagation();
                            cfg_role = `personal`;
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
        </div>
    </div>
    <div
        class={`absolute max-m_0:bottom-0 bottom-10 left-0 flex flex-col w-full justify-center items-center`}
    >
        <ButtonLayoutPair
            basis={{
                continue: {
                    label: `${$ls(`common.continue`)}`,
                    disabled:
                        (cfg_profile_name_skip &&
                            cfg_profile_nip05_opt &&
                            !cfg_profile_name_valid) ||
                        ($carousel_index === 1 && !cfg_role),
                    callback: async () => await handle_continue(),
                },
                back: {
                    visible: true,
                    label: cfg_profile_name_skip
                        ? `${$ls(`common.skip`)}`
                        : `${$ls(`common.back`)}`,
                    callback: handle_back,
                },
            }}
        />
    </div>
</div>
<div
    data-view={`eula`}
    class={`hidden flex flex-col h-full w-full max-m_0:pt-12 pt-24 justify-start items-center`}
>
    <div
        data-carousel-container={`eula`}
        class={`carousel-container flex h-full w-full rounded-2xl scroll-hide`}
    >
        <div
            data-carousel-item={`eula`}
            class={`carousel-item flex flex-col w-full max-m_0:pt-16 justify-start items-center`}
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
                        onscroll={on_scroll_eula}
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
                    class={`flex flex-row w-full pt-6 justify-center items-center`}
                >
                    <button
                        class={`group flex flex-row basis-1/2 gap-4 justify-center items-center ${el_eula_scrolled ? `` : `opacity-80`}`}
                        onclick={async () => {
                            const confirm = await gui.confirm({
                                message: `${$ls(`eula.error.required`)}`,
                                cancel: `${$ls(`common.quit`)}`,
                            });
                            if (confirm === false) await reset(undefined, true);
                        }}
                    >
                        <p
                            class={`font-mono font-[400] text-sm text-layer-0-glyph group-active:text-layer-0-glyph el-re`}
                        >
                            {`-`}
                        </p>
                        <p
                            class={`font-mono font-[400] text-sm text-layer-0-glyph group-active:text-layer-0-glyph el-re`}
                        >
                            {`${`${$ls(`common.disagree`)}`}`}
                        </p>
                        <p
                            class={`font-mono font-[400] text-sm text-layer-0-glyph group-active:text-layer-0-glyph el-re`}
                        >
                            {`-`}
                        </p>
                    </button>
                    <button
                        class={`relative group flex flex-row basis-1/2 gap-4 justify-center items-center el-re ${el_eula_scrolled ? `` : `opacity-40`}`}
                        onclick={async () => {
                            if (el_eula_scrolled) await submit();
                        }}
                    >
                        <p
                            class={`font-mono font-[400] text-sm text-layer-0-glyph-hl group-active:text-layer-0-glyph-hl/80 el-re`}
                        >
                            {`-`}
                        </p>
                        <p
                            class={`font-mono font-[400] text-sm text-layer-0-glyph-hl group-active:text-layer-0-glyph-hl/80 el-re`}
                        >
                            {`${`${$ls(`common.agree`)}`}`}
                        </p>
                        <p
                            class={`font-mono font-[400] text-sm text-layer-0-glyph-hl group-active:text-layer-0-glyph-hl/80 el-re`}
                        >
                            {`- `}
                        </p>
                        {#if loading_submit}
                            <div
                                class={`absolute right-3 flex flex-row justify-start items-center`}
                            >
                                <LoadSymbol basis={{ dim: `xs` }} />
                            </div>
                        {/if}
                    </button>
                </div>
            </div>
        </div>
    </div>
</div>
