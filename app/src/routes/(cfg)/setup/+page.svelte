<script lang="ts">
    import { goto } from "$app/navigation";
    import { datastore, db, nostr_keys, notif } from "$lib/utils/app";
    import { route } from "$lib/utils/app/app";
    import { reset_sql_cipher } from "$lib/utils/app/cipher";
    import {
        cfg_delay,
        type AppData,
        type ConfigData,
    } from "$lib/utils/config";
    import { ls } from "$lib/utils/i18n";
    import { get_default_nostr_relays } from "$lib/utils/nostr/lib";
    import { NDKPrivateKeySigner } from "@nostr-dev-kit/ndk";
    import {
        carousel_dec,
        carousel_inc,
        casl_i,
        casl_imax,
        el_id,
        Fade,
        fmt_id,
        Glyph,
        sleep,
        view_effect,
    } from "@radroots/apps-lib";
    import {
        ButtonLayoutPair,
        EntryLine,
        LoadSymbol,
        LogoCircle,
        SelectMenu,
    } from "@radroots/apps-lib-pwa";
    import { app_lo, app_loading } from "@radroots/apps-lib-pwa/stores/app";
    import type { AppConfigRole } from "@radroots/apps-lib-pwa/types/app";
    import type { IError } from "@radroots/types-bindings";
    import {
        err_msg,
        form_fields,
        handle_err,
        type ResultPass,
    } from "@radroots/utils";
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

    type View = "cfg_key" | "cfg_profile" | "eula";
    let view: View = $state("cfg_key");
    $effect(() => {
        view_effect<View>(view);
    });

    let cfg_role: AppConfigRole | undefined = $state(undefined);
    type CfgKeyOpt = "nostr_key_gen" | "nostr_key_add";
    let cgf_key_opt: CfgKeyOpt | undefined = $state(undefined);

    let nostr_key_add_val = $state(``);

    let profile_name_val = $state(``);
    let profile_name_valid = $state(false);
    let profile_name_nip05 = $state(false);
    let profile_name_loading = $state(false);

    let is_eula_scrolled = $state(false);
    let is_loading_s = $state(false);

    onMount(async () => {
        try {
            await page_init();
        } catch (e) {
            handle_err(e, `on_mount`);
        }
    });

    const page_init = async (): Promise<void> => {
        const nostr_keys_all = await nostr_keys.keys();
        if ("results" in nostr_keys_all) {
            const confirm = await notif.confirm({
                message: `Clear the prior session?`,
            });
            if (!confirm) {
                alert("@todo add the prior session");
                return;
            }
            await page_reset();
        }
        handle_view(view);
    };

    const page_reset = async (
        alert_message?: string,
        prevent_loading?: boolean,
    ): Promise<void> => {
        try {
            console.log(`[config] page reset`);
            app_loading.set(!prevent_loading);
            handle_view(`cfg_key`);
            if (alert_message) await notif.alert(alert_message);
            await sleep(cfg_delay.load);
            await nostr_keys.reset();
            await datastore.reset();
            await reset_sql_cipher(db.get_store_key());
            await db.reinit();
        } catch (e) {
            handle_err(e, `reset`);
        } finally {
            app_loading.set(false);
        }
    };

    const on_scroll_eula = async ({
        currentTarget: el,
    }: {
        currentTarget: EventTarget & HTMLDivElement;
    }): Promise<void> => {
        const client_h = el?.clientHeight;
        const scroll_h = el?.scrollHeight;
        const scroll_top = el?.scrollTop;
        if (scroll_top + client_h >= scroll_h) is_eula_scrolled = true;
    };

    const handle_view = (new_view: View): void => {
        if (new_view === "cfg_key" && view === "cfg_profile") {
            const offset = cgf_key_opt === "nostr_key_gen" ? 1 : 0;
            casl_i.set(page_carousel[new_view].max_index - offset);
        } else {
            casl_i.set(0);
            casl_imax.set(page_carousel[new_view].max_index);
        }
        view = new_view;
    };

    const handle_config_err = async (
        err?: IError<string> | string,
    ): Promise<void> => {
        console.log(`err `, err);
        let message = `${$ls(`error.init.configuration_failure`)}`;
        if (typeof err === "string") message = err;
        else if (err && "err" in err) message = err.err;
        await page_reset(message);
    };

    const handle_continue = async (): Promise<void> => {
        switch (view) {
            case `cfg_key`:
                switch ($casl_i) {
                    case 0:
                        return await carousel_inc(view);
                    case 1:
                        return handle_new_key_or_add();
                    case 2:
                        return handle_key_add_existing();
                }
            case `cfg_profile`:
                switch ($casl_i) {
                    case 0:
                        return handle_setup_profile();
                    case 1:
                        return handle_setup_role();
                }
        }
    };

    const handle_new_key_or_add = async (): Promise<void> => {
        try {
            if (cgf_key_opt === `nostr_key_add`)
                return void (await carousel_inc(view));
            await create_nostr_key();
            handle_view(`cfg_profile`);
        } catch (e) {
            handle_err(e, `handle_new_key_or_add`);
        }
    };

    const create_nostr_key = async (): Promise<void> => {
        const keys_nostr_gen = await nostr_keys.generate();
        if (`err` in keys_nostr_gen) return handle_config_err();
        await datastore.update_obj<ConfigData>("cfg_data", {
            nostr_public_key: keys_nostr_gen.public_key,
        });
    };

    const add_nostr_key = async (secret_key: string): Promise<void> => {
        const keys_nostr_add = await nostr_keys.add(secret_key);
        if ("err" in keys_nostr_add)
            return void (await notif.alert(`${$ls(`common.invalid_key`)}`));
        await datastore.update_obj<ConfigData>("cfg_data", {
            nostr_public_key: keys_nostr_add.public_key,
        });
    };

    const handle_key_add_existing = async (): Promise<void> => {
        try {
            if (!nostr_key_add_val)
                return void (await notif.alert(
                    `${$ls(`icu.enter_a_*`, {
                        value: `${$ls(`common.nostr_key`)}`.toLowerCase(),
                    })}`,
                ));
            const key_add_signer = new NDKPrivateKeySigner(nostr_key_add_val);
            await add_nostr_key(key_add_signer.privateKey);
            nostr_key_add_val = ``;
            handle_view(`cfg_profile`);
        } catch (e) {
            handle_err(e, `handle_key_add_existing`);
            return void (await notif.alert(
                `${$ls(`icu.not_a_valid_*`, {
                    value: `${$ls(`common.nostr_key`)}`.toLowerCase(),
                })}`,
            ));
        }
    };

    const handle_setup_profile = async (): Promise<void> => {
        try {
            if (profile_name_loading) return;

            const ds_cfg_data = await datastore.get_obj<ConfigData>("cfg_data");
            if ("err" in ds_cfg_data) return handle_config_err();

            const ks_nostr_key = await nostr_keys.read(
                ds_cfg_data.result.nostr_public_key,
            );
            if ("err" in ks_nostr_key) return handle_config_err();

            if (profile_name_nip05) {
                if (!profile_name_val)
                    return void (await notif.alert(
                        `${$ls(`icu.enter_a_*`, {
                            value: `${$ls(
                                `common.profile_name`,
                            )}`.toLowerCase(),
                        })}`,
                    ));
                if (!profile_name_valid)
                    return void (await notif.alert(
                        `Profile name must be at least 3 characters`, //@todo
                    ));
                await datastore.update_obj<ConfigData>("cfg_data", {
                    nip05_request: true,
                });
                // @todo add nip-05 request
            }

            if (!profile_name_val) {
                const confirm = handle_add_profile_without_name_confirmation();
                if (!confirm)
                    return void el_id(fmt_id(`nostr:profile`))?.focus();
            }

            if (profile_name_val) {
                await datastore.update_obj<ConfigData>("cfg_data", {
                    nostr_profile: profile_name_val,
                });
            }

            await carousel_inc(view);
        } catch (e) {
            handle_err(e, `handle_setup_profile`);
        } finally {
            profile_name_loading = false;
        }
    };

    const handle_add_profile_without_name_confirmation =
        async (): Promise<boolean> => {
            return await notif.confirm({
                message: `${$ls(`notification.init.no_profile_option`)}`,
                cancel: `${$ls(`icu.add_*`, {
                    value: `${$ls(`common.profile`)}`,
                })}`,
                ok: `${$ls(`common.continue`)}`,
            });
        };

    const handle_setup_role = async (): Promise<void> => {
        if (!cfg_role) cfg_role = `personal`;
        await datastore.update_obj<ConfigData>("cfg_data", { role: cfg_role });
        handle_view(`eula`);
    };

    const handle_back = async (): Promise<void> => {
        switch (view) {
            case `cfg_key`:
                switch ($casl_i) {
                    case 1: {
                        cgf_key_opt = undefined;
                        return await carousel_dec(view);
                    }
                    case 2: {
                        nostr_key_add_val = ``;
                        return await carousel_dec(view);
                    }
                }
            case `cfg_profile`:
                switch ($casl_i) {
                    case 0: {
                        if (!profile_name_val) {
                            const confirm =
                                await handle_add_profile_without_name_confirmation();
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
            is_loading_s = true;
            const ds_cfg_data = await datastore.get_obj<ConfigData>("cfg_data");
            if ("err" in ds_cfg_data) return handle_config_err(ds_cfg_data);
            const active_key = ds_cfg_data.result?.nostr_public_key;
            if (!active_key)
                return handle_config_err(`${$ls(`error.init.no_active_key`)}`);
            const ks_nostr_key = await nostr_keys.read(active_key);
            if ("err" in ks_nostr_key) return handle_config_err(ks_nostr_key);
            const configure_result = await configure_device(
                active_key,
                ds_cfg_data.result.role || "personal",
                ds_cfg_data.result.nostr_profile,
            );
            if ("err" in configure_result) {
                return void (await notif.alert(configure_result.err));
            } else if (!("pass" in configure_result)) {
                return void (await notif.alert(
                    `${$ls(`error.init.configuration_failure`)}`,
                ));
            }
            const clear_cfg = await datastore.del_obj("cfg_data");
            if ("err" in clear_cfg) return handle_config_err(clear_cfg);
            goto(`/`);
            await notif.notify_init();
        } catch (e) {
            handle_err(e, `submit`);
        } finally {
            is_loading_s = false;
        }
    };

    const configure_device = async (
        public_key: string,
        role: AppConfigRole,
        profile_name?: string,
    ): Promise<ResultPass | IError<string>> => {
        const nostr_profile_add = await db.nostr_profile_create({
            public_key,
            name: profile_name ? profile_name : "default",
            display_name: profile_name ? profile_name : undefined,
        });
        if ("err" in nostr_profile_add)
            return err_msg(
                `${$ls(`error.init.device_configuration_nostr_profile`)}`,
            );
        for (const url of get_default_nostr_relays()) {
            const nostr_relay_add = await db.nostr_relay_create({ url });
            if ("err" in nostr_relay_add)
                return err_msg(
                    `${$ls(`error.init.device_configuration_nostr_relay`)}`,
                );
            const nostr_profile_relay_set = await db.nostr_profile_relay_set({
                nostr_profile: {
                    id: nostr_profile_add.result.id,
                },
                nostr_relay: {
                    id: nostr_relay_add.result.id,
                },
            });
            if ("err" in nostr_profile_relay_set)
                return err_msg(
                    `${$ls(
                        `error.init.device_configuration_nostr_profile_relay`,
                    )}`,
                );
        }
        await datastore.del_obj("cfg_data");
        await datastore.set_obj<AppData>("app_data", {
            active_key: public_key,
            role,
            eula_date: new Date().toISOString(),
        });
        return { pass: true };
    };
</script>

{#if view === "cfg_key" && $casl_i === 1}
    <Fade basis={{ classes: `z-10 absolute top-8 right-6` }}>
        <SelectMenu
            basis={{
                layer: 0,
                options: [
                    {
                        entries: [
                            {
                                value: "",
                                label: "Choose Options",
                                disabled: true,
                            },
                            {
                                value: "import",
                                label: "Import Backup",
                            },
                        ],
                    },
                ],
                callback: async ({ value }) => {
                    if (value === "import")
                        return void (await route("/import"));
                },
            }}
        >
            <Glyph
                basis={{
                    classes: `text-base text-ly0-gl group-active:text-ly0-gl-a`,
                    key: "gear",
                }}
            />
        </SelectMenu>
    </Fade>
{/if}

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
                            class={`font-sans font-[400] text-sm text-ly0-gl-label uppercase`}
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
                                class={`font-mono font-[400] text-[1.1rem] text-ly0-gl`}
                            >
                                {`${$ls(`notification.init.greeting_header`)}`}
                            </p>
                        </div>
                        <div
                            class={`flex flex-row w-full justify-start items-center`}
                        >
                            <p
                                class={`font-mono font-[400] text-[1.1rem] text-ly0-gl`}
                            >
                                {`${$ls(
                                    `notification.init.greeting_subheader`,
                                )}.`}
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
                    <p class={`font-sans font-[600] text-ly0-gl text-3xl`}>
                        {`${$ls(`icu.configure_*`, {
                            value: `${$ls(`common.device`)}`,
                        })}`}
                    </p>
                </div>
                <div
                    class={`flex flex-col w-full gap-6 justify-center items-center`}
                >
                    <button
                        class={`flex flex-col h-bold_button w-lo_${$app_lo} justify-center items-center rounded-touch ${
                            cgf_key_opt === `nostr_key_gen`
                                ? `ly1-apply-active ly1-raise-apply ly1-ring-apply`
                                : `bg-ly1`
                        } el-re`}
                        onclick={async (ev) => {
                            ev.stopPropagation();
                            cgf_key_opt = `nostr_key_gen`;
                        }}
                    >
                        <p class={`font-sans font-[600] text-ly0-gl text-xl`}>
                            {`${$ls(`icu.create_new_*`, {
                                value: `${$ls(`common.keypair`)}`.toLowerCase(),
                            })}`}
                        </p>
                    </button>
                    <button
                        class={`flex flex-col h-bold_button w-lo_${$app_lo} justify-center items-center rounded-touch ${
                            cgf_key_opt === `nostr_key_add`
                                ? `ly1-apply-active ly1-raise-apply ly1-ring-apply`
                                : `bg-ly1`
                        } el-re`}
                        onclick={async (ev) => {
                            ev.stopPropagation();
                            cgf_key_opt = `nostr_key_add`;
                        }}
                    >
                        <p class={`font-sans font-[600] text-ly0-gl text-xl`}>
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
                        class={`font-sans font-[600] text-ly0-gl text-3xl capitalize`}
                    >
                        {`${$ls(`icu.add_existing_*`, {
                            value: `${$ls(`common.key`)}`.toLowerCase(),
                        })}`}
                    </p>
                    <EntryLine
                        bind:value={nostr_key_add_val}
                        basis={{
                            wrap: {
                                layer: 1,
                                classes: `w-lo_${$app_lo}`,
                                style: `guide`,
                            },
                            el: {
                                classes: `font-sans text-[1.25rem] text-center placeholder:opacity-60`,
                                layer: 1,
                                placeholder: `${$ls(`icu.enter_*`, {
                                    value: `nostr nsec/hex`,
                                })}`,
                                callback_keydown: async ({ key_s, el }) => {
                                    if (key_s) {
                                        el.blur();
                                        handle_continue();
                                    }
                                },
                            },
                        }}
                    />
                </div>
            </div>
        </div>
        <div
            class={`z-10 absolute ios0:bottom-2 bottom-10 left-0 flex flex-col w-full justify-center items-center`}
        >
            <ButtonLayoutPair
                basis={{
                    continue: {
                        label: `${$ls(`common.continue`)}`,
                        disabled: $casl_i === 1 && !cgf_key_opt,
                        callback: async () => handle_continue(),
                    },
                    back: {
                        label: `${$ls(`common.back`)}`,
                        visible: $casl_i > 0,
                        callback: async () => handle_back(),
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
                <p class={`font-sans font-[600] text-ly0-gl text-3xl`}>
                    {`${$ls(`icu.add_*`, {
                        value: `${$ls(`common.profile`)}`,
                    })}`}
                </p>
                <div
                    class={`flex flex-col w-full gap-4 justify-center items-center`}
                >
                    <EntryLine
                        bind:value={profile_name_val}
                        basis={{
                            loading: profile_name_loading,
                            wrap: {
                                layer: 1,
                                classes: `w-lo_${$app_lo}`,
                                style: `guide`,
                            },
                            el: {
                                classes: `font-sans text-[1.25rem] text-center placeholder:opacity-60`,
                                id: fmt_id(`nostr:profile`),
                                layer: 1,
                                placeholder: `${$ls(`icu.enter_*`, {
                                    value: `${$ls(
                                        `common.profile_name`,
                                    )}`.toLowerCase(),
                                })}`,
                                field: form_fields.profile_name,
                                callback: async ({ pass }) => {
                                    profile_name_valid = pass;
                                },
                                callback_keydown: async ({ key_s, el }) => {
                                    if (key_s) {
                                        el.blur();
                                        handle_continue();
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
                            bind:checked={profile_name_nip05}
                        />
                        <button
                            class={`flex flex-row justify-center items-center`}
                            onclick={async () => {
                                profile_name_nip05 = !profile_name_nip05;
                            }}
                        >
                            <p
                                class={`font-sans font-[400] text-ly0-gl text-[14px] tracking-wide`}
                            >
                                {`${$ls(`common.create`)}`}
                                <span
                                    class={`font-mono font-[500] tracking-tight px-[3px]`}
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
                    <p class={`font-sans font-[600] text-ly0-gl text-3xl`}>
                        {`${$ls(`common.setup_for_farmer`)}`}
                    </p>
                </div>
                <div
                    class={`flex flex-col w-full gap-5 justify-center items-center`}
                >
                    <button
                        class={`flex flex-col h-bold_button w-lo_${$app_lo} justify-center items-center rounded-touch ${
                            cfg_role === `farmer`
                                ? `ly1-apply-active ly1-raise-apply ly1-ring-apply`
                                : `bg-ly1`
                        } el-re`}
                        onclick={async (ev) => {
                            ev.stopPropagation();
                            cfg_role = `farmer`;
                        }}
                    >
                        <p class={`font-sans font-[600] text-ly0-gl text-xl`}>
                            {`${$ls(`common.yes`)}`}
                        </p>
                    </button>
                    <button
                        class={`flex flex-col h-bold_button w-lo_${$app_lo} justify-center items-center rounded-touch ${
                            cfg_role === `personal`
                                ? `ly1-apply-active ly1-raise-apply ly1-ring-apply`
                                : `bg-ly1`
                        } el-re`}
                        onclick={async (ev) => {
                            ev.stopPropagation();
                            cfg_role = `personal`;
                        }}
                    >
                        <p class={`font-sans font-[600] text-ly0-gl text-xl`}>
                            {`${$ls(`common.no`)}`}
                        </p>
                    </button>
                </div>
            </div>
        </div>
    </div>
    <div
        class={`absolute ios0:bottom-2 bottom-10 left-0 flex flex-col w-full justify-center items-center`}
    >
        <ButtonLayoutPair
            basis={{
                continue: {
                    label: `${$ls(`common.continue`)}`,
                    disabled: $casl_i === 1 && !cfg_role,
                    callback: async () => handle_continue(),
                },
                back: {
                    visible: true,
                    label:
                        view === "cfg_profile" &&
                        $casl_i === 0 &&
                        !profile_name_val
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
    class={`hidden flex flex-col h-full w-full ios0:pt-12 pt-24 justify-start items-center`}
>
    <div
        data-carousel-container={`eula`}
        class={`carousel-container flex h-full w-full rounded-2xl scroll-hide`}
    >
        <div
            data-carousel-item={`eula`}
            class={`carousel-item flex flex-col h-full w-full justify-start items-center`}
        >
            <div
                class={`flex flex-col h-full w-full px-4 justify-start items-center ${
                    view === `eula` ? `fade-in-long` : ``
                }`}
            >
                <div
                    class={`flex flex-col w-full px-4 gap-4 justify-start items-center`}
                >
                    <div
                        class={`flex flex-row w-full ios0:pt-8 justify-center items-center`}
                    >
                        <p class={`font-mono font-[600] text-ly0-gl text-2xl`}>
                            {`${$ls(`eula.title`)}`}
                        </p>
                    </div>
                    <div
                        class={`flex flex-col ios0:h-[26rem] ios1:h-[38rem] w-full gap-6 justify-start items-center overflow-y-scroll scroll-hide`}
                        onscroll={on_scroll_eula}
                    >
                        <div
                            class={`flex flex-col w-full gap-2 justify-start items-start`}
                        >
                            <p class={`font-mono font-[600] text-ly0-gl`}>
                                {`**${$ls(`eula.introduction.title`)}**`}
                            </p>
                            <p
                                class={`font-mono font-[500] text-ly0-gl text-sm text-justify break-word`}
                            >
                                {`${$ls(`eula.introduction.body`)}`}
                            </p>
                        </div>
                        <div
                            class={`flex flex-col w-full gap-2 justify-start items-start`}
                        >
                            <p class={`font-mono font-[600] text-ly0-gl`}>
                                {`**${$ls(`eula.prohibited_content.title`)}**`}
                            </p>
                            <p
                                class={`font-mono font-[500] text-sm text-ly0-gl text-justify break-word`}
                            >
                                {`${$ls(
                                    `eula.prohibited_content.body_0_title`,
                                )}`}
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
                                                class={` font-mono font-[500] text-sm text-ly0-gl text-justify break-word`}
                                            >
                                                {`*`}
                                            </p>
                                        </div>
                                        <div
                                            class={`flex flex-row h-full w-full justify-start items-start`}
                                        >
                                            <p
                                                class={`col-span-10 font-mono font-[500] text-sm text-ly0-gl text-justify break-word`}
                                            >
                                                {`${$ls(
                                                    `eula.prohibited_content.body_li_0_${li}`,
                                                )}`}
                                            </p>
                                        </div>
                                    </div>
                                {/each}
                            </div>
                        </div>
                        <div
                            class={`flex flex-col w-full gap-2 justify-start items-start`}
                        >
                            <p class={`font-mono font-[600] text-ly0-gl`}>
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
                                                class={` font-mono font-[500] text-sm text-ly0-gl text-justify break-word`}
                                            >
                                                {`*`}
                                            </p>
                                        </div>
                                        <div
                                            class={`flex flex-row h-full w-full justify-start items-start`}
                                        >
                                            <p
                                                class={`col-span-10 font-mono font-[500] text-sm text-ly0-gl text-justify break-word`}
                                            >
                                                {`${$ls(
                                                    `eula.prohibited_conduct.body_li_0_${li}`,
                                                )}`}
                                            </p>
                                        </div>
                                    </div>
                                {/each}
                            </div>
                        </div>
                        <div
                            class={`flex flex-col w-full gap-2 justify-start items-start`}
                        >
                            <p class={`font-mono font-[600] text-ly0-gl`}>
                                {`**${$ls(
                                    `eula.consequences_of_violation.title`,
                                )}**`}
                            </p>
                            <p
                                class={`font-mono font-[500] text-ly0-gl text-sm text-justify break-word`}
                            >
                                {`${$ls(
                                    `eula.consequences_of_violation.body`,
                                )}`}
                            </p>
                        </div>
                        <div
                            class={`flex flex-col w-full gap-2 justify-start items-start`}
                        >
                            <p class={`font-mono font-[600] text-ly0-gl`}>
                                {`**${$ls(`eula.disclaimer.title`)}**`}
                            </p>
                            <p
                                class={`font-mono font-[500] text-ly0-gl text-sm text-justify break-word`}
                            >
                                {`${$ls(`eula.disclaimer.body`)}`}
                            </p>
                        </div>
                        <div
                            class={`flex flex-col w-full gap-2 justify-start items-start`}
                        >
                            <p class={`font-mono font-[600] text-ly0-gl`}>
                                {`**${$ls(`eula.changes.title`)}**`}
                            </p>
                            <p
                                class={`font-mono font-[500] text-ly0-gl text-sm text-justify break-word`}
                            >
                                {`${$ls(`eula.changes.body`)}`}
                            </p>
                        </div>
                        <div
                            class={`flex flex-col w-full gap-2 justify-start items-start`}
                        >
                            <p class={`font-mono font-[600] text-ly0-gl`}>
                                {`**${$ls(`eula.contact.title`)}**`}
                            </p>
                            <p
                                class={`font-mono font-[500] text-ly0-gl text-sm text-justify break-word`}
                            >
                                {`${$ls(`eula.contact.body`)}`}
                            </p>
                        </div>
                        <div
                            class={`flex flex-col w-full gap-2 justify-start items-start`}
                        >
                            <p class={`font-mono font-[600] text-ly0-gl`}>
                                {`**${$ls(`eula.acceptance_of_terms.title`)}**`}
                            </p>
                            <p
                                class={`font-mono font-[500] text-ly0-gl text-sm text-justify break-word`}
                            >
                                {`${$ls(`eula.acceptance_of_terms.body`)}`}
                            </p>
                        </div>
                    </div>
                </div>
                <div
                    class={`flex flex-row w-full ios0:pt-8 pt-6 justify-center items-center`}
                >
                    <button
                        class={`group flex flex-row basis-1/2 gap-4 justify-center items-center ${
                            is_eula_scrolled ? `` : `opacity-80`
                        }`}
                        onclick={async () => {
                            const confirm = await notif.confirm({
                                message: `${$ls(`eula.error.required`)}`,
                                cancel: `${$ls(`common.quit`)}`,
                            });

                            if (confirm === false)
                                await page_reset(undefined, true);
                        }}
                    >
                        <p
                            class={`font-mono font-[400] text-sm text-ly0-gl group-active:text-ly0-gl el-re`}
                        >
                            {`-`}
                        </p>
                        <p
                            class={`font-mono font-[400] text-sm text-ly0-gl group-active:text-ly0-gl el-re`}
                        >
                            {`${`${$ls(`common.disagree`)}`}`}
                        </p>
                        <p
                            class={`font-mono font-[400] text-sm text-ly0-gl group-active:text-ly0-gl el-re`}
                        >
                            {`-`}
                        </p>
                    </button>
                    <button
                        class={`relative group flex flex-row basis-1/2 gap-4 justify-center items-center el-re ${
                            is_eula_scrolled ? `` : `opacity-40`
                        }`}
                        onclick={async () => {
                            if (is_eula_scrolled) await submit();
                        }}
                    >
                        <p
                            class={`font-mono font-[400] text-sm text-ly0-gl-hl group-active:text-ly0-gl-hl/80 el-re`}
                        >
                            {`-`}
                        </p>
                        <p
                            class={`font-mono font-[400] text-sm text-ly0-gl-hl group-active:text-ly0-gl-hl/80 el-re`}
                        >
                            {`${`${$ls(`common.agree`)}`}`}
                        </p>
                        <p
                            class={`font-mono font-[400] text-sm text-ly0-gl-hl group-active:text-ly0-gl-hl/80 el-re`}
                        >
                            {`- `}
                        </p>
                        {#if is_loading_s}
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
