<script lang="ts">
    import { goto } from "$app/navigation";
    import { PUBLIC_NOSTR_RELAY_DEFAULTS } from "$env/static/public";
    import { ls } from "$lib/locale/i18n";
    import {
        datastore,
        db,
        gui,
        keys,
        nostrkey,
        radroots,
        route,
    } from "$lib/util";
    import { cfg_delay } from "$lib/util/conf";
    import {
        app_lo,
        app_loading,
        app_notify,
        ButtonLayoutPair,
        carousel_dec,
        carousel_inc,
        carousel_index,
        carousel_index_max,
        EntryLineIdb,
        fmt_id,
        handle_err,
        idb,
        IdbLib,
        Input,
        LabelDisplay,
        LoadSymbol,
        LogoCircle,
        view_effect,
    } from "@radroots/lib-app";
    import {
        el_id,
        form_fields,
        sleep,
        str_capitalize_words,
        type ResultPass,
    } from "@radroots/util";
    import { onMount } from "svelte";

    type IdbKey = `nostr:key:add` | `nostr:profile` | `#key_nostrp`;
    const kv = new IdbLib<IdbKey>(idb);

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

    type CfgKeyOpt = `cfg_keygen` | `cfg_keyadd`;
    let cgf_keyopt: CfgKeyOpt | undefined = $state(undefined);

    type CfgRole = `farmer` | `personal`;
    let cfg_role: CfgRole | undefined = $state(undefined);

    let cfg_profile_nostr_publickey = $state(``);
    const cfg_profile_nostr_publickey_npub = $derived(
        cfg_profile_nostr_publickey
            ? nostrkey.npub(cfg_profile_nostr_publickey) || ``
            : ``,
    );

    let cfg_profile_profilename_valid = $state(false);
    let cfg_profile_profilename_loading = $state(false);

    let loading_submit = $state(false);

    onMount(async () => {
        try {
            await init();
        } catch (e) {
            handle_err(e, `on_mount`);
        }
    });

    const init = async (): Promise<void> => {
        const nostrkey_all = await keys.nostr_read_all();
        if (`results` in nostrkey_all && nostrkey_all.results.length) {
            console.log(`init EXISTING!`, nostrkey_all.results);
            handle_view(`eula`);
        } else {
            handle_view(view);
        }
        await kv.init();
    };

    const handle_view = (new_view: View): void => {
        console.log(`new_view `, new_view);
        if (new_view === `cfg_key` && view === `cfg_profile`) {
            const offset = cgf_keyopt === `cfg_keygen` ? 1 : 0;
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
        await kv.save(`#key_nostrp`, keys_nostr_create.public_key);
    };

    const key_add = async (secret_key: string): Promise<void> => {
        const keys_nostr_add = await keys.nostr_add(secret_key);
        if (`err` in keys_nostr_add)
            return void (await gui.alert(`${$ls(`common.invalid_key`)}`));
        await kv.save(`#key_nostrp`, keys_nostr_add.public_key);
    };

    const configure_device = async (
        public_key: string,
        profile_name?: string,
    ): Promise<ResultPass | void> => {
        const nostr_profile_add = await db.nostr_profile_create({
            public_key,
            name: profile_name ? profile_name : undefined,
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

    const handle_choose_key_gen_or_add = async (): Promise<void> => {
        try {
            if (cgf_keyopt === `cfg_keyadd`)
                return void (await carousel_inc(view));
            await key_gen();
            handle_view(`cfg_profile`);
        } catch (e) {
            await handle_err(e, `handle_choose_key_gen_or_add`);
        }
    };

    const handle_submit_key_add = async (): Promise<void> => {
        try {
            const nostrkey_add = await kv.read(`nostr:key:add`);
            if (!nostrkey_add)
                return void (await gui.alert(
                    `${$ls(`icu.enter_a_valid_*`, { value: `${$ls(`common.key`)}` })}`,
                ));
            const key_nostr_read = await keys.nostr_read(nostrkey_add);
            if (`err` in key_nostr_read)
                return void (await reset(
                    `${$ls(`error.init.configuration_failure`)}`,
                ));
            await key_add(key_nostr_read.secret_key);
            kv.del(`nostr:key:add`);
            return void handle_view(`cfg_profile`);
        } catch (e) {
            await handle_err(e, `handle_submit_key_add`);
        }
    };

    const handle_profile_add = async (): Promise<void> => {
        try {
            if (cfg_profile_profilename_loading) return;
            const kv_keynostrp = await kv.read(`#key_nostrp`);
            if (!kv_keynostrp)
                return void (await reset(
                    `${$ls(`error.init.configuration_failure`)}`,
                )); //@todo
            const key_nostr_read = await keys.nostr_read(kv_keynostrp);
            if (`err` in key_nostr_read)
                return void (await reset(
                    `${$ls(`error.init.configuration_failure`)}`,
                )); //@todo
            const kv_profilename = await kv.read(`nostr:profile`);
            if (!kv_profilename)
                return void (await gui.alert(
                    `${$ls(`icu.enter_a_*`, { value: `${$ls(`common.profile_name`)}`.toLowerCase() })}`,
                ));
            cfg_profile_profilename_loading = true;
            const profile_req = await radroots.fetch_profile_request({
                profile_name: kv_profilename,
                secret_key: key_nostr_read.secret_key,
            });
            if (`err` in profile_req)
                return void (await gui.alert(
                    `${$ls(profile_req.err, { default: `${$ls(`error.client.http.request_failure`)}` })}`,
                ));
            const confirm = await gui.confirm({
                message: `${`${$ls(`icu.the_*_is_available`, { value: `${$ls(`common.profile_name`).toLowerCase()} "${kv_profilename}"` })}`}. ${`${$ls(`common.would_you_like_to_use_it_q`)}`}`,
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
                kv_keynostrp,
                profile_create.result,
            );
            await carousel_inc(view);
        } catch (e) {
            await handle_err(e, `handle_profile_add`);
        } finally {
            cfg_profile_profilename_loading = false;
        }
    };

    const handle_set_role = async (): Promise<void> => {
        if (!cfg_role) cfg_role = `personal`;
        await datastore.set(`role`, cfg_role);
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
                        return await handle_submit_key_add();
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
                        cgf_keyopt = undefined;
                        return await carousel_dec(view);
                    }
                    case 2:
                        return await carousel_dec(view);
                }
            case `cfg_profile`:
                switch ($carousel_index) {
                    case 0:
                        return handle_view(`cfg_key`);
                    case 1:
                        return carousel_dec(view);
                }
        }
    };

    const submit = async (): Promise<void> => {
        try {
            loading_submit = true;
            const kv_keynostrp = await kv.read(`#key_nostrp`);
            if (!kv_keynostrp)
                return void (await reset(
                    `${$ls(`error.init.configuration_failure`)}`,
                )); //@todo
            const key_nostr_read = await keys.nostr_read(kv_keynostrp);
            if (`err` in key_nostr_read)
                return void (await reset(
                    `${$ls(`error.init.configuration_failure`)}`,
                )); //@todo
            const radroots_profile = await datastore.getp(
                `radroots_profile`,
                kv_keynostrp,
            );
            if (`result` in radroots_profile) {
                await radroots.fetch_profile_activate({
                    id: radroots_profile.result,
                    secret_key: key_nostr_read.secret_key,
                }); //@todo
            }
            const configuration_result = await configure_device(
                kv_keynostrp,
                await kv.read(`nostr:profile`),
            );
            if (configuration_result && `pass` in configuration_result) {
                const confirm = await gui.confirm({
                    message: `${$ls(`notification.init.on_complete`)}`,
                    ok: `${$ls(`common.continue`)}`,
                    cancel: str_capitalize_words(
                        `${$ls(`common.hide_alerts`)}`,
                    ),
                });
                if (confirm) {
                    await gui.notify_init();
                    app_notify.set(`${$ls(`notification.init.on_first_load`)}`);
                }
                await route(`/`);
            }
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
        class={`z-10 absolute max-m_0:bottom-0 bottom-10 left-0 flex flex-col w-full justify-center items-center`}
    >
        <ButtonLayoutPair
            basis={{
                continue: {
                    label: `${$ls(`common.continue`)}`,
                    disabled: $carousel_index === 1 && !cgf_keyopt,
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
                cgf_keyopt = undefined;
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
                        class={`flex flex-col h-bold_button w-lo_${$app_lo} justify-center items-center rounded-touch ${cgf_keyopt === `cfg_keygen` ? `layer-1-surface-apply-active layer-1-raise-apply layer-1-ring-apply` : `bg-layer-1-surface`} el-re`}
                        onclick={async (ev) => {
                            ev.stopPropagation();
                            cgf_keyopt = `cfg_keygen`;
                        }}
                    >
                        <p
                            class={`font-sans font-[600] text-layer-0-glyph text-xl`}
                        >
                            {`${$ls(`icu.create_new_*`, { value: `${$ls(`common.keypair`)}`.toLowerCase() })}`}
                        </p>
                    </button>
                    <button
                        class={`flex flex-col h-bold_button w-lo_${$app_lo} justify-center items-center rounded-touch ${cgf_keyopt === `cfg_keyadd` ? `layer-1-surface-apply-active layer-1-raise-apply layer-1-ring-apply` : `bg-layer-1-surface`} el-re`}
                        onclick={async (ev) => {
                            ev.stopPropagation();
                            cgf_keyopt = `cfg_keyadd`;
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
                    {#if cfg_profile_nostr_publickey}
                        <p
                            class={`font-sans font-[600] text-layer-0-glyph text-3xl`}
                        >
                            {`${$ls(`common.using_public_key`)}`}
                        </p>
                        <LabelDisplay
                            basis={{
                                classes: `w-lo_${$app_lo}`,
                                label: {
                                    classes: `pl-4 font-mono text-lg text-start truncate`,
                                    value:
                                        cfg_profile_nostr_publickey_npub ||
                                        cfg_profile_nostr_publickey,
                                },
                                style: `guide`,
                            }}
                        />
                    {:else}
                        <p
                            class={`font-sans font-[600] text-layer-0-glyph text-3xl capitalize`}
                        >
                            {`${$ls(`icu.add_existing_*`, { value: `${$ls(`common.key`)}`.toLowerCase() })}`}
                        </p>
                        <Input
                            basis={{
                                classes: `h-entry_guide w-lo_${$app_lo} bg-layer-1-surface layer-1-focus-surface rounded-touch font-mono text-lg placeholder:opacity-60 items-end text-center`,
                                id: fmt_id(`nostr:key:add`),
                                sync: true,
                                placeholder: `${$ls(`icu.enter_*`, { value: `nostr nsec/hex` })}`,
                                field: form_fields.profile_name,
                                callback_keydown: async ({ key_s, el }) => {
                                    if (key_s) {
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
                class={`flex flex-col h-[16rem] w-full px-4 gap-10 justify-start items-center`}
            >
                <p class={`font-sans font-[600] text-layer-0-glyph text-3xl`}>
                    {`${$ls(`icu.add_*`, { value: `${$ls(`common.profile`)}` })}`}
                </p>
                <EntryLineIdb
                    basis={{
                        loading: cfg_profile_profilename_loading,
                        wrap: {
                            layer: 1,
                            classes: `w-lo_${$app_lo}`,
                            style: `guide`,
                        },
                        el: {
                            classes: `font-sans text-[1.25rem] text-center placeholder:opacity-60`,
                            id: fmt_id(`nostr:profile`),
                            sync: true,
                            layer: 1,
                            placeholder: `${$ls(`icu.enter_*`, { value: `${$ls(`common.profile_name`)}`.toLowerCase() })}`,
                            field: form_fields.profile_name,
                            callback: async ({ pass }) => {
                                cfg_profile_profilename_valid = pass;
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
                        ($carousel_index === 0 &&
                            !cfg_profile_profilename_valid) ||
                        ($carousel_index === 1 && !cfg_role),
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
                            const confirm = await gui.confirm({
                                message: `${$ls(`notification.init.no_profile_option`)}`,
                                cancel: `${$ls(`icu.add_*`, { value: `${$ls(`common.profile`)}` })}`,
                                ok: `${$ls(`common.continue`)}`,
                            });
                            if (confirm === false)
                                return void el_id(
                                    fmt_id(`nostr:profile`),
                                )?.focus();
                            return void carousel_inc(view);
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
