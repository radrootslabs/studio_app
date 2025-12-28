<script lang="ts">
    import { goto } from "$app/navigation";
    import {
        datastore,
        db,
        nostr_keys,
        notif,
        radroots,
        route,
    } from "$lib/utils/app";
    import { reset_sql_cipher } from "$lib/utils/app/cipher";
    import {
        cfg_delay,
        type AppData,
        type ConfigData,
    } from "$lib/utils/config";
    import { ls } from "$lib/utils/i18n";
    import { get_default_nostr_relays } from "$lib/utils/nostr/lib";
    import {
        carousel_create,
        carousel_dec,
        carousel_inc,
        carousel_init,
        el_id,
        Fade,
        fmt_id,
        Glyph,
        sleep,
        ViewPane,
        ViewStack,
    } from "@radroots/apps-lib";
    import {
        ButtonLayoutPair,
        CarouselContainer,
        CarouselItem,
        EntryLine,
        LoadSymbol,
        LogoCircle,
        SelectMenu,
    } from "@radroots/apps-lib-pwa";
    import { app_lo, app_loading } from "@radroots/apps-lib-pwa/stores/app";
    import type { AppConfigRole } from "@radroots/apps-lib-pwa/types/app";
    import { nostr_secret_key_validate } from "@radroots/nostr";
    import type { IError } from "@radroots/types-bindings";
    import {
        err_msg,
        form_fields,
        handle_err,
        type ResultPass,
    } from "@radroots/utils";
    import { onMount } from "svelte";

    type View = "cfg_key" | "cfg_profile" | "eula";

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

    const carousel_cfg_key = carousel_create({
        view: "cfg_key",
        max_index: page_carousel.cfg_key.max_index,
    });

    const carousel_cfg_profile = carousel_create({
        view: "cfg_profile",
        max_index: page_carousel.cfg_profile.max_index,
    });

    const carousel_eula = carousel_create({
        view: "eula",
        max_index: page_carousel.eula.max_index,
    });

    const view_carousel = {
        cfg_key: carousel_cfg_key,
        cfg_profile: carousel_cfg_profile,
        eula: carousel_eula,
    };

    const carousel_cfg_profile_index = carousel_cfg_profile.index;

    let view: View = $state("cfg_key");

    let cfg_role: AppConfigRole | undefined = $state(undefined);
    type CfgKeyOpt = "nostr_key_gen" | "nostr_key_add";
    let cgf_key_opt: CfgKeyOpt | undefined = $state(undefined);

    type CfgKeyStep = "intro" | "choice" | "add_existing";
    let cfg_key_step: CfgKeyStep = $state("intro");
    let cfg_key_loading = $state(false);

    const cfg_key_step_index = (step: CfgKeyStep): number => {
        switch (step) {
            case "intro":
                return 0;
            case "choice":
                return 1;
            case "add_existing":
                return 2;
        }
    };

    $effect(() => {
        console.log(`view `, view);
        console.log(`cfg_key_step `, cfg_key_step);
    });

    const cfg_key_step_for_index = (index: number): CfgKeyStep => {
        if (index <= 0) return "intro";
        if (index === 1) return "choice";
        return "add_existing";
    };

    let nostr_key_add_val = $state(``);

    let profile_name_val = $state(``);
    let profile_name_valid = $state(false);
    let profile_name_nip05 = $state(true);
    let profile_name_loading = $state(false);

    let is_eula_scrolled = $state(false);
    let is_loading_s = $state(false);

    const reset_local_state = (): void => {
        cfg_role = undefined;
        cgf_key_opt = undefined;
        cfg_key_step = "intro";
        cfg_key_loading = false;
        nostr_key_add_val = ``;
        profile_name_val = ``;
        profile_name_valid = false;
        profile_name_nip05 = true;
        profile_name_loading = false;
        is_eula_scrolled = false;
        is_loading_s = false;
    };

    const sync_carousel = (new_view: View, index: number): void => {
        const carousel = view_carousel[new_view];
        carousel_init(carousel, {
            index,
            max_index: page_carousel[new_view].max_index,
        });
    };

    const set_cfg_key_step = (step: CfgKeyStep): void => {
        cfg_key_step = step;
        // @todo confirm why this was breaking the correct... if (step === "choice" && !cgf_key_opt) cgf_key_opt = "nostr_key_gen";
        sync_carousel("cfg_key", cfg_key_step_index(step));
    };

    onMount(async () => {
        try {
            await page_init();
        } catch (e) {
            handle_err(e, `on_mount`);
        }
    });

    const page_init = async (): Promise<void> => {
        reset_local_state();
        const nostr_keys_all = await nostr_keys.keys();
        if ("results" in nostr_keys_all) {
            const confirm = await notif.confirm({
                message: `${$ls(
                    `notification.configuration.clear_prior_session`,
                )}`,
            });
            if (!confirm) {
                await notif.alert(
                    `${$ls(
                        `notification.configuration.prior_session_pending`,
                    )}`,
                );
                return;
            }
            await page_reset();
            return;
        }
        handle_view(`cfg_key`, { index: 0 });
    };

    const page_reset = async (
        alert_message?: string,
        prevent_loading?: boolean,
    ): Promise<void> => {
        try {
            console.log(`[config] page reset`);
            app_loading.set(!prevent_loading);
            reset_local_state();
            handle_view(`cfg_key`, { index: 0 });
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

    const handle_view = (new_view: View, opts?: { index?: number }): void => {
        let next_index = opts?.index ?? 0;
        if (new_view === "cfg_key") {
            if (opts?.index !== undefined)
                cfg_key_step = cfg_key_step_for_index(opts.index);
            else if (view === "cfg_profile")
                cfg_key_step =
                    cgf_key_opt === "nostr_key_add" ? "add_existing" : "choice";
            if (cfg_key_step === "choice" && !cgf_key_opt)
                cgf_key_opt = "nostr_key_gen";
            next_index = cfg_key_step_index(cfg_key_step);
        }
        view = new_view;
        sync_carousel(new_view, next_index);
    };

    $effect(() => {
        console.log(`cgf_key_opt `, cgf_key_opt);
    });

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
                switch (cfg_key_step) {
                    case "intro":
                        return set_cfg_key_step("choice");
                    case "choice":
                        return handle_new_key_or_add();
                    case "add_existing":
                        return handle_key_add_existing();
                }
            case `cfg_profile`:
                switch ($carousel_cfg_profile_index) {
                    case 0:
                        return handle_setup_profile();
                    case 1:
                        return handle_setup_role();
                }
        }
    };

    const handle_new_key_or_add = async (): Promise<void> => {
        console.log(`RUNNING NOSTR KEY SETUP `, cgf_key_opt);
        if (cgf_key_opt === `nostr_key_add`)
            return set_cfg_key_step("add_existing");
        if (cfg_key_loading) return;
        cfg_key_loading = true;
        try {
            console.log(`cfg_key_gen start`, {
                view,
                cfg_key_step,
                cgf_key_opt,
            });
            const key_created = await create_nostr_key();
            console.log(`cfg_key_gen result`, { key_created });
            if (!key_created) return;
            handle_view(`cfg_profile`);
            console.log(`cfg_key_gen view`, { view });
        } catch (e) {
            console.log(`ERR `, e);
            handle_err(e, `handle_new_key_or_add`);
        } finally {
            cfg_key_loading = false;
        }
    };

    const create_nostr_key = async (): Promise<boolean> => {
        console.log(`cfg_key_gen keystore generate start`);
        const keys_nostr_gen = await nostr_keys.generate();
        console.log(`keys_nostr_gen `, keys_nostr_gen);
        if ("err" in keys_nostr_gen) {
            console.log(`cfg_key_gen keystore generate err`, keys_nostr_gen);
            await handle_config_err(keys_nostr_gen);
            return false;
        }
        console.log(`cfg_key_gen keystore generate ok`);
        const cfg_update = await datastore.update_obj<ConfigData>("cfg_data", {
            nostr_public_key: keys_nostr_gen.public_key,
        });
        console.log(`cfg_update `, cfg_update);
        if ("err" in cfg_update) {
            console.log(`cfg_key_gen datastore update err`, cfg_update);
            await handle_config_err(cfg_update);
            return false;
        }
        console.log(`cfg_key_gen datastore update ok`);
        return true;
    };

    const add_nostr_key = async (secret_key: string): Promise<boolean> => {
        console.log(`cfg_key_add keystore add start`);
        const keys_nostr_add = await nostr_keys.add(secret_key);
        if ("err" in keys_nostr_add) {
            console.log(`cfg_key_add keystore add err`, keys_nostr_add);
            await notif.alert(`${$ls(`common.invalid_key`)}`);
            return false;
        }
        console.log(`cfg_key_add keystore add ok`);
        const cfg_update = await datastore.update_obj<ConfigData>("cfg_data", {
            nostr_public_key: keys_nostr_add.public_key,
        });
        if ("err" in cfg_update) {
            console.log(`cfg_key_add datastore update err`, cfg_update);
            await handle_config_err(cfg_update);
            return false;
        }
        console.log(`cfg_key_add datastore update ok`);
        return true;
    };

    const handle_key_add_existing = async (): Promise<void> => {
        if (cfg_key_loading) return;
        let loading_set = false;
        try {
            if (!nostr_key_add_val)
                return void (await notif.alert(
                    `${$ls(`icu.enter_a_*`, {
                        value: `${$ls(`common.nostr_key`)}`.toLowerCase(),
                    })}`,
                ));
            const secret_key = nostr_secret_key_validate(nostr_key_add_val);
            if (!secret_key)
                return void (await notif.alert(
                    `${$ls(`icu.not_a_valid_*`, {
                        value: `${$ls(`common.nostr_key`)}`.toLowerCase(),
                    })}`,
                ));
            cfg_key_loading = true;
            loading_set = true;
            console.log(`cfg_key_add start`, {
                view,
                cfg_key_step,
            });
            const key_added = await add_nostr_key(secret_key);
            console.log(`cfg_key_add result`, { key_added });
            if (!key_added) return;
            nostr_key_add_val = ``;
            handle_view(`cfg_profile`);
            console.log(`cfg_key_add view`, { view });
        } catch (e) {
            handle_err(e, `handle_key_add_existing`);
            return void (await notif.alert(
                `${$ls(`icu.not_a_valid_*`, {
                    value: `${$ls(`common.nostr_key`)}`.toLowerCase(),
                })}`,
            ));
        } finally {
            if (loading_set) cfg_key_loading = false;
        }
    };

    const handle_setup_profile = async (): Promise<void> => {
        try {
            if (profile_name_loading) return;
            //@
            const ds_cfg_data = await datastore.get_obj<ConfigData>("cfg_data");
            console.log(`ds_cfg_data `, ds_cfg_data);
            if ("err" in ds_cfg_data) return handle_config_err();

            const ks_nostr_key = await nostr_keys.read(
                ds_cfg_data.result.nostr_public_key,
            );
            console.log(`ks_nostr_key `, ks_nostr_key);
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
                        `${$ls(`error.configuration.profile.name_min_length`)}`,
                    ));
                profile_name_loading = true;
                const accounts_req = await radroots.accounts_request({
                    profile_name: profile_name_val,
                    secret_key: ks_nostr_key.secret_key,
                });
                if ("err" in accounts_req)
                    return void (await notif.alert(
                        `${$ls(accounts_req.err, {
                            default: `${$ls(
                                `error.client.http.request_failure`,
                            )}`,
                        })}`,
                    ));
                const confirm = await notif.confirm({
                    message: `${`${$ls(`icu.the_*_is_available`, {
                        value: `${$ls(
                            `common.profile_name`,
                        ).toLowerCase()} "${profile_name_val}"`,
                    })}`}. ${`${$ls(`common.would_you_like_to_use_it_q`)}`}`,
                    cancel: `${$ls(`common.no`)}`,
                    ok: `${$ls(`common.yes`)}`,
                });
                if (!confirm) return;
                const accounts_create = await radroots.accounts_create({
                    tok: accounts_req.result,
                    secret_key: ks_nostr_key.secret_key,
                });
                if ("err" in accounts_create)
                    return void (await notif.alert(
                        `${$ls(accounts_create.err, {
                            default: `${$ls(
                                `error.client.http.request_failure`,
                            )}`,
                        })}`,
                    ));
                await datastore.update_obj<ConfigData>("cfg_data", {
                    nip05_request: true,
                    nip05_key: accounts_create.result,
                });
            }

            if (!profile_name_val) {
                const confirm =
                    await handle_add_profile_without_name_confirmation();
                if (!confirm)
                    return void el_id(fmt_id(`nostr:profile`))?.focus();
            }

            if (profile_name_val) {
                await datastore.update_obj<ConfigData>("cfg_data", {
                    nostr_profile: profile_name_val,
                });
            }

            await carousel_inc(view_carousel[view]);
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
        if (!cfg_role) cfg_role = `individual`;
        await datastore.update_obj<ConfigData>("cfg_data", { role: cfg_role });
        handle_view(`eula`);
    };

    const handle_back = async (): Promise<void> => {
        if (cfg_key_loading) return;
        switch (view) {
            case `cfg_key`:
                switch (cfg_key_step) {
                    case "choice": {
                        cgf_key_opt = undefined;
                        return set_cfg_key_step("intro");
                    }
                    case "add_existing": {
                        nostr_key_add_val = ``;
                        return set_cfg_key_step("choice");
                    }
                }
            case `cfg_profile`:
                switch ($carousel_cfg_profile_index) {
                    case 0: {
                        if (!profile_name_val) {
                            const confirm =
                                await handle_add_profile_without_name_confirmation();
                            if (!confirm)
                                return void el_id(
                                    fmt_id(`nostr:profile`),
                                )?.focus();
                            return void carousel_inc(view_carousel[view]);
                        }
                        return handle_view(`cfg_key`);
                    }
                    case 1:
                        return carousel_dec(view_carousel[view]);
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
                ds_cfg_data.result,
                //ds_cfg_data.result.role || "personal",
                //ds_cfg_data.result.nostr_profile,
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
        config_data: ConfigData,
    ): Promise<ResultPass | IError<string>> => {
        const profile_type =
            config_data.role === `farmer`
                ? `individual`
                : config_data.role ?? `individual`;
        const nostr_profile_add = await db.nostr_profile_create({
            public_key,
            profile_type,
            name: config_data.nostr_profile
                ? config_data.nostr_profile
                : `${$ls(`common.default`)}`,
            display_name: config_data.nostr_profile
                ? config_data.nostr_profile
                : undefined,
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
        const set_app_data = await datastore.set_obj<AppData>("app_data", {
            active_key: public_key,
            role: config_data.role || "individual",
            eula_date: new Date().toISOString(),
            nip05_key: config_data.nip05_key,
        });
        if ("err" in set_app_data) return err_msg(set_app_data);
        await datastore.del_obj("cfg_data");
        return { pass: true };
    };

    $effect(() => {
        console.log(`view `, view);
    });
</script>

{#if view === "cfg_key" && cfg_key_step !== "intro"}
    <Fade basis={{ classes: `z-10 absolute top-8 right-6` }}>
        <SelectMenu
            basis={{
                layer: 0,
                options: [
                    {
                        entries: [
                            {
                                value: "",
                                label: `${$ls(`common.choose_options`)}`,
                                disabled: true,
                            },
                            {
                                value: "import",
                                label: `${$ls(`common.import_backup`)}`,
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

<ViewStack
    basis={{
        active_view: view,
    }}
>
    <ViewPane basis={{ view: "cfg_key" }}>
        <div class={`flex flex-col h-full w-full justify-start items-center`}>
            <CarouselContainer
                basis={{
                    carousel: carousel_cfg_key,
                }}
            >
                <CarouselItem
                    basis={{
                        classes: `justify-center items-center`,
                    }}
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
                </CarouselItem>
                <CarouselItem
                    basis={{
                        classes: `justify-center items-center`,
                    }}
                >
                    <div
                        class={`flex flex-col h-[16rem] gap-8 w-full justify-start items-center`}
                    >
                        <div
                            class={`flex flex-row w-full justify-center items-center`}
                        >
                            <p
                                class={`font-sans font-[600] text-ly0-gl text-3xl`}
                            >
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
                                <p
                                    class={`font-sans font-[600] text-ly0-gl text-xl`}
                                >
                                    {`${$ls(`icu.create_new_*`, {
                                        value: `${$ls(
                                            `common.keypair`,
                                        )}`.toLowerCase(),
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
                                <p
                                    class={`font-sans font-[600] text-ly0-gl text-xl`}
                                >
                                    {`${$ls(`icu.use_existing_*`, {
                                        value: `${$ls(
                                            `common.keypair`,
                                        )}`.toLowerCase(),
                                    })}`}
                                </p>
                            </button>
                        </div>
                    </div>
                </CarouselItem>
                <CarouselItem
                    basis={{
                        classes: `justify-center items-center`,
                    }}
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
                                            value: `${$ls(
                                                `common.nostr_nsec_hex`,
                                            )}`,
                                        })}`,
                                        callback_keydown: async ({
                                            key_s,
                                            el,
                                        }) => {
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
                </CarouselItem>
                <div
                    class={`z-10 absolute ios0:bottom-2 bottom-10 left-0 flex flex-col w-full justify-center items-center`}
                >
                    <ButtonLayoutPair
                        basis={{
                            continue: {
                                label: `${$ls(`common.continue`)}`,
                                disabled:
                                    cfg_key_loading ||
                                    (cfg_key_step === "choice" && !cgf_key_opt),
                                loading: cfg_key_loading,
                                callback: async () => handle_continue(),
                            },
                            back: {
                                label: `${$ls(`common.back`)}`,
                                visible: cfg_key_step !== "intro",
                                disabled: cfg_key_loading,
                                callback: async () => handle_back(),
                            },
                        }}
                    />
                </div>
            </CarouselContainer>
        </div>
    </ViewPane>

    <ViewPane basis={{ view: "cfg_profile" }}>
        <div class={`flex flex-col h-full w-full justify-start items-center`}>
            <CarouselContainer
                basis={{
                    carousel: carousel_cfg_profile,
                }}
            >
                <CarouselItem
                    basis={{
                        classes: `justify-center items-center`,
                    }}
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
                                        callback_keydown: async ({
                                            key_s,
                                            el,
                                        }) => {
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
                                        profile_name_nip05 =
                                            !profile_name_nip05;
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
                </CarouselItem>
                <CarouselItem
                    basis={{
                        classes: `justify-center items-center`,
                        role: `button`,
                        tabindex: 0,
                        callback_click: async () => {
                            cfg_role = undefined;
                        },
                    }}
                >
                    <div
                        class={`flex flex-col h-[16rem] w-full gap-10 justify-start items-center`}
                    >
                        <div
                            class={`flex flex-row w-full justify-center items-center`}
                        >
                            <p
                                class={`font-sans font-[600] text-ly0-gl text-3xl`}
                            >
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
                                <p
                                    class={`font-sans font-[600] text-ly0-gl text-xl`}
                                >
                                    {`${$ls(`common.yes`)}`}
                                </p>
                            </button>
                            <button
                                class={`flex flex-col h-bold_button w-lo_${$app_lo} justify-center items-center rounded-touch ${
                                    cfg_role === `individual`
                                        ? `ly1-apply-active ly1-raise-apply ly1-ring-apply`
                                        : `bg-ly1`
                                } el-re`}
                                onclick={async (ev) => {
                                    ev.stopPropagation();
                                    cfg_role = `individual`;
                                }}
                            >
                                <p
                                    class={`font-sans font-[600] text-ly0-gl text-xl`}
                                >
                                    {`${$ls(`common.no`)}`}
                                </p>
                            </button>
                        </div>
                    </div>
                </CarouselItem>
            </CarouselContainer>
            <div
                class={`absolute ios0:bottom-2 bottom-10 left-0 flex flex-col w-full justify-center items-center`}
            >
                <ButtonLayoutPair
                    basis={{
                        continue: {
                            label: `${$ls(`common.continue`)}`,
                            disabled:
                                $carousel_cfg_profile_index === 1 && !cfg_role,
                            callback: async () => handle_continue(),
                        },
                        back: {
                            visible: true,
                            label:
                                view === "cfg_profile" &&
                                $carousel_cfg_profile_index === 0 &&
                                !profile_name_val
                                    ? `${$ls(`common.skip`)}`
                                    : `${$ls(`common.back`)}`,
                            callback: handle_back,
                        },
                    }}
                />
            </div>
        </div>
    </ViewPane>

    <ViewPane basis={{ view: "eula" }}>
        <div
            class={`flex flex-col h-full w-full ios0:pt-12 pt-24 justify-start items-center`}
        >
            <CarouselContainer
                basis={{
                    carousel: carousel_eula,
                    classes: `rounded-2xl scroll-hide`,
                }}
            >
                <CarouselItem
                    basis={{
                        classes: `justify-start items-center`,
                    }}
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
                                <p
                                    class={`font-mono font-[600] text-ly0-gl text-2xl`}
                                >
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
                                    <p
                                        class={`font-mono font-[600] text-ly0-gl`}
                                    >
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
                                    <p
                                        class={`font-mono font-[600] text-ly0-gl`}
                                    >
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
                                    <p
                                        class={`font-mono font-[600] text-ly0-gl`}
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
                                    <p
                                        class={`font-mono font-[600] text-ly0-gl`}
                                    >
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
                                    <p
                                        class={`font-mono font-[600] text-ly0-gl`}
                                    >
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
                                    <p
                                        class={`font-mono font-[600] text-ly0-gl`}
                                    >
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
                                    <p
                                        class={`font-mono font-[600] text-ly0-gl`}
                                    >
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
                                    <p
                                        class={`font-mono font-[600] text-ly0-gl`}
                                    >
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
                                        message: `${$ls(
                                            `eula.error.required`,
                                        )}`,
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
                </CarouselItem>
            </CarouselContainer>
        </div>
    </ViewPane>
</ViewStack>
