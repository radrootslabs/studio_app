<script lang="ts">
    import {
        PUBLIC_NOSTR_RELAY_DEFAULTS,
        PUBLIC_RADROOTS_URL,
    } from "$env/static/public";
    import { lc } from "$lib/client";
    import { _conf } from "$lib/conf";
    import { restart } from "$lib/utils";
    import { keystore_reset } from "$lib/utils/keystore";
    import {
        app_layout,
        carousel_index,
        carousel_index_max,
        carousel_next,
        carousel_prev,
        Fill,
        fmt_capitalize,
        fmt_id,
        Glyph,
        InputElement,
        kv,
        LayoutView,
        sleep,
        t,
        view_effect,
        type GlyphKey,
        type GlyphWeight,
    } from "@radroots/svelte-lib";
    import { regex } from "@radroots/utils";
    import { onMount } from "svelte";

    type KeypairNewButtonKey =
        | `keypair-profile-personal`
        | `keypair-profile-wallet`
        | `keypair-profile-links`;
    type KeypairNewButton = {
        key: KeypairNewButtonKey;
        icon: GlyphKey;
        weight?: GlyphWeight;
        heading: string;
        subtitle: string;
    };
    const keypair_new_buttons: KeypairNewButton[] = [
        {
            key: `keypair-profile-personal`,
            icon: `user`,
            heading: `${$t(`common.personal`)}`,
            subtitle: [
                `${$t(`common.your_name`)}`,
                `${$t(`common.business_name`)}`,
            ].join(`, `),
        },
        {
            key: `keypair-profile-wallet`,
            icon: `cardholder`,
            heading: `${$t(`common.wallet`)}`,
            subtitle: [
                `${$t(`common.bank_account`)}`,
                fmt_capitalize(`lud06`),
                fmt_capitalize(`lud16`),
            ].join(`, `),
        },
        {
            key: `keypair-profile-links`,
            icon: `globe-simple`,
            weight: `bold`,
            heading: `Links`,
            subtitle: [
                `${$t(`common.website`)}`,
                `${$t(`common.socials`)}`,
                `${$t(`common.photos`)}`,
            ].join(`, `),
        },
    ];

    let keypair_new_buttons_selected: KeypairNewButtonKey[] = [];

    const carousel_param: Record<View, { max: number }> = {
        start: {
            max: 1,
        },
        setup: {
            max: 1,
        },
        keypair_new: {
            max: 1,
        },
        keypair_own: {
            max: 1,
        },
        eula: {
            max: 1,
        },
    };

    onMount(async () => {
        try {
            carousel_index.set(0);
            carousel_index_max.set(carousel_param[view].max);
            await keystore_reset();
        } catch (e) {
        } finally {
        }
    });

    type View = `start` | `setup` | `keypair_new` | `keypair_own` | `eula`;
    let view: View = `setup`;

    $: {
        view_effect<View>(view);
    }

    const handle_carousel_prev = async (): Promise<void> => {
        try {
            await carousel_prev(view);
        } catch (e) {
            console.log(`(error) handle_carousel_prev `, e);
        }
    };

    const handle_carousel_next = async (): Promise<void> => {
        try {
            switch (view) {
                case `start`:
                    {
                        await carousel_next(view);
                    }
                    break;
                case `setup`: {
                    switch ($carousel_index) {
                        case 0:
                            {
                                await carousel_next(view);
                            }
                            break;
                        case 1:
                            {
                                const profile_name = await kv.get(
                                    fmt_id(`profile_name`),
                                );
                                if (!profile_name) {
                                    await lc.dialog.alert(
                                        `${$t(`icu.enter_a_*`, { value: `${$t(`common.profile_name`)}`.toLowerCase() })}`,
                                    );
                                    return;
                                }

                                const valid_profile_name =
                                    await validate_profile_name(profile_name);
                                if (typeof valid_profile_name === `string`) {
                                    await lc.dialog.alert(valid_profile_name);
                                    return;
                                } else if (!valid_profile_name) {
                                    await lc.dialog.alert(
                                        `${`${$t(`icu.the_*_is_registered`, { value: `${$t(`common.profile_name`)}` })}`}`,
                                    );
                                    return;
                                }

                                const confirm = await lc.dialog.confirm({
                                    message: `${`${$t(`icu.the_*_is_available`, { value: `${$t(`common.profile_name`).toLowerCase()} ${profile_name}` })}`}`,
                                    cancel_label: `Back`,
                                    ok_label: `Continue`,
                                });
                                if (confirm === true) {
                                    await carousel_next(view);
                                }
                            }
                            break;
                    }
                }
            }
        } catch (e) {
            console.log(`(error) handle_carousel_next `, e);
        }
    };

    const validate_profile_name = async (
        profile_name: string,
    ): Promise<boolean | string> => {
        try {
            const res = await lc.http.fetch({
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
            console.log(`(error) validate_profile_name `, e);
            return `catch`;
        }
    };

    const configure_device = async (): Promise<void> => {
        try {
            const secret_key = lc.nostr.lib.generate_key();
            const public_key = lc.nostr.lib.public_key(secret_key);

            const ks_key_add = await lc.keystore.set(
                _conf.kv.nostr_key(public_key),
                secret_key,
            );
            if (!ks_key_add) {
                alert(`!ks_key_add`);
                return;
                //@todo reset
            }

            const pref_key_add = await lc.preferences.set(
                _conf.kv.nostr_key_active,
                public_key,
            );
            if (!pref_key_add) {
                alert(`!pref_key_add`);
                return;
                //@todo reset
            }

            const nostr_profile_add = await lc.db.nostr_profile_add({
                public_key,
            });

            if (`err` in nostr_profile_add) {
                await lc.dialog.alert(nostr_profile_add.err);
                return;
                //@todo
            } else if (`err_s` in nostr_profile_add) {
                await lc.dialog.alert(nostr_profile_add.err_s.join(` `));
                return;
                //@todo
            }

            for (const url of PUBLIC_NOSTR_RELAY_DEFAULTS.split(",") || []) {
                const nostr_relay_add = await lc.db.nostr_relay_add({ url });
                if (`err` in nostr_relay_add) {
                    await lc.dialog.alert(nostr_relay_add.err);
                    return;
                    // @todo
                } else if (`err_s` in nostr_relay_add) {
                    await lc.dialog.alert(nostr_relay_add.err_s.join(` `));
                    return;
                    //@todo
                }
                await lc.db.set_nostr_profile_relay({
                    nostr_profile: {
                        id: nostr_profile_add.id,
                    },
                    nostr_relay: {
                        id: nostr_relay_add.id,
                    },
                });
            }

            await sleep(_conf.delay.load);
            await restart(
                true,
                `Welcome! Your device was configured. To view or change your configuration go to Settings > Configuration.`,
            );
        } catch (e) {
            console.log(`(error) configure_device `, e);
        }
    };

    const handle_keypair_buttons_select = (key: KeypairNewButtonKey): void => {
        if (keypair_new_buttons_selected.includes(key)) {
            keypair_new_buttons_selected = keypair_new_buttons_selected.filter(
                (i) => i !== key,
            );
        } else {
            keypair_new_buttons_selected = [
                ...keypair_new_buttons_selected,
                key,
            ];
        }
    };
</script>

<LayoutView>
    <div
        data-view={`start`}
        class={`hidden flex flex-col h-full w-full py-12 justify-start items-center`}
    >
        <div
            class={`relative flex flex-col h-full w-full justify-start items-center`}
        >
            <div
                data-carousel-container={`start`}
                class={`carousel-container flex h-full w-full rounded-2xl scrollbar-hide`}
            >
                <div
                    data-carousel-item={`start`}
                    class={`carousel-item flex flex-col w-full py-32 justify-between items-center`}
                >
                    <div
                        class={`flex flex-col gap-2 justify-center items-center`}
                    >
                        <p
                            class={`font-mono font-[600] text-layer-0-glyph text-6xl`}
                        >
                            {`rad`}
                        </p>
                        <button
                            class={`flex flex-row justify-center items-center`}
                            on:click={async () => {
                                //@todo remove
                                await configure_device();
                            }}
                        >
                            <p
                                class={`font-mono font-[600] text-layer-0-glyph text-6xl`}
                            >
                                {`roots`}
                            </p>
                        </button>
                    </div>
                    <div class={`flex flex-col justify-start items-center`}>
                        <p
                            class={`font-mono font-[400] text-layer-0-glyph text-3xl`}
                        >
                            {`direct trade`}
                        </p>
                        <p
                            class={`font-mono font-[400] text-layer-0-glyph text-3xl`}
                        >
                            {`network`}
                        </p>
                    </div>
                </div>
            </div>
            <div
                class={`absolute bottom-0 left-0 flex flex-row h-12 w-full justify-center items-center`}
            >
                <div class={`flex flex-row gap-8 justify-start items-center`}>
                    <Glyph
                        basis={{
                            key: `caret-left`,
                            dim: `sm-`,
                            weight: `bold`,
                            classes:
                                $carousel_index > 0
                                    ? `h-glyph_btn_sm w-glyph_btn_sm text-layer-2-glyph bg-layer-2-surface rounded-full`
                                    : `h-glyph_btn_sm w-glyph_btn_sm text-layer-2-glyph bg-layer-2-surface rounded-full opacity-60`,
                            callback: async () => {
                                if ($carousel_index > 0)
                                    await handle_carousel_prev();
                            },
                        }}
                    />
                    <Glyph
                        basis={{
                            key: `caret-right`,
                            dim: `sm-`,
                            weight: `bold`,
                            classes: `h-glyph_btn_sm w-glyph_btn_sm text-layer-2-glyph bg-layer-2-surface rounded-full`,
                            callback: async () => {
                                if (
                                    $carousel_index !== carousel_param[view].max
                                )
                                    await handle_carousel_next();
                                else view = `setup`;
                            },
                        }}
                    />
                </div>
            </div>
        </div>
    </div>
    <div
        data-view={`setup`}
        class={`flex flex-col h-full w-full max-mobile_base:pt-12 pt-16 justify-start items-center`}
    >
        <div
            class={`relative flex flex-col h-full w-full justify-start items-center`}
        >
            <div
                data-carousel-container={`setup`}
                class={`carousel-container flex h-full w-full rounded-2xl scrollbar-hide`}
            >
                <div
                    data-carousel-item={`setup`}
                    class={`carousel-item flex flex-col w-full max-mobile_y:pt-28 pt-32 pb-4 justify-start items-center`}
                >
                    <div
                        class={`flex flex-col gap-8 justify-start items-center`}
                    >
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
                            {#each [`Choose a profile name`, `Configure your device`, `Terms of Use agreement`] as li, li_i}
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
                <div
                    data-carousel-item={`setup`}
                    class={`carousel-item flex flex-col w-full max-mobile_y:pt-32 pt-36 pb-4 justify-start items-center`}
                >
                    <div
                        class={`flex flex-col w-full gap-8 justify-start items-center`}
                    >
                        <div
                            class={`flex flex-row w-full justify-center items-center`}
                        >
                            <p
                                class={`font-mono font-[600] text-layer-0-glyph text-3xl`}
                            >
                                {`2. ${`${$t(`common.device`)}`}`}
                            </p>
                        </div>
                        <div
                            class={`flex flex-col w-full gap-4 justify-center items-center`}
                        >
                            <button
                                class={`flex flex-col h-[4.25rem] w-${$app_layout} justify-center items-center bg-layer-1-surface rounded-3xl touch-layer-1 touch-layer-1-raise-less`}
                                on:click={async () => {
                                    view = `keypair_new`;
                                }}
                            >
                                <p
                                    class={`font-sans font-[600] text-layer-0-glyph text-xl`}
                                >
                                    {`Create secure keypair`}
                                </p>
                            </button>
                            <button
                                class={`flex flex-col h-[4.25rem] w-${$app_layout} justify-center items-center bg-layer-1-surface rounded-3xl touch-layer-1 touch-layer-1-raise-less`}
                                on:click={async () => {
                                    view = `keypair_own`;
                                }}
                            >
                                <p
                                    class={`font-sans font-[600] text-layer-0-glyph text-xl`}
                                >
                                    {`Use existing keypair`}
                                </p>
                            </button>
                        </div>
                    </div>
                    <div
                        class={`absolute bottom-0 left-0 flex flex-col w-full justify-center items-center ${$carousel_index > 0 ? `pb-2` : `pb-4`}`}
                    >
                        <button
                            class={`group flex flex-row h-[3.25rem] w-${$app_layout} justify-center items-center bg-layer-1-surface rounded-touch touch-layer-1`}
                            on:click={async () => {
                                await handle_carousel_next();
                            }}
                        >
                            <p
                                class={`font-sans font-[600] tracking-wide text-layer-1-glyph/80 group-active:text-layer-1-glyph/40 transition-all`}
                            >
                                {`${$t(`common.continue`)}`}
                            </p>
                        </button>
                        <div
                            class={`flex flex-col justify-center items-center transition-all`}
                        >
                            {#if $carousel_index > 0}
                                <button
                                    class={`group flex flex-row h-12 w-${$app_layout} justify-center items-center fade-in`}
                                    on:click={async () => {
                                        await handle_carousel_prev();
                                    }}
                                >
                                    <p
                                        class={`font-sans font-[600] tracking-wide text-layer-1-glyph-shade group-active:text-layer-1-glyph/40 transition-all`}
                                    >
                                        {`${$t(`common.back`)}`}
                                    </p>
                                </button>
                            {:else}
                                <div
                                    class={`flex flex-row h-1 w-full justify-start items-center`}
                                >
                                    <Fill />
                                </div>
                            {/if}
                        </div>
                    </div>
                </div>
                <div
                    data-carousel-item={`setup`}
                    class={`carousel-item flex flex-col w-full max-mobile_y:pt-32 pt-36 pb-4 justify-start items-center`}
                >
                    <div
                        class={`flex flex-col w-full gap-8 justify-start items-center`}
                    >
                        <div
                            class={`flex flex-row w-full justify-center items-center`}
                        >
                            <p
                                class={`font-mono font-[600] text-layer-0-glyph text-3xl`}
                            >
                                {`1. ${`${$t(`common.profile`)}`}`}
                            </p>
                        </div>
                        <div
                            class={`flex flex-col h-36 w-full py-4 px-4 gap-5 justify-start items-center bg-layer-1-surface rounded-3xl`}
                        >
                            <div
                                class={`flex flex-row w-full justify-center items-center`}
                            >
                                <p
                                    class={`font-sans font-[600] text-layer-0-glyph text-xl`}
                                >
                                    {`Choose a profile name`}
                                </p>
                            </div>
                            <div
                                class={`flex flex-row w-full justify-center items-center border-b-line border-b-layer-1-surface-edge`}
                            >
                                <InputElement
                                    basis={{
                                        classes: `h-10 font-mono placeholder:opacity-60 items-end text-center`,
                                        id: fmt_id(`profile_name`),
                                        sync: true,
                                        sync_init: true,
                                        placeholder: `Enter a profile name`,
                                        field: {
                                            charset: regex.profile_name_ch,
                                            validate: regex.profile_name,
                                            validate_keypress: true,
                                        },
                                        callback_keydown: async ({
                                            key,
                                            el,
                                        }) => {
                                            if (key === `Enter`) {
                                                el.blur();
                                                await handle_carousel_next();
                                            }
                                        },
                                    }}
                                />
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </div>
    <div
        data-view={`keypair_new`}
        class={`hidden flex flex-col h-full w-full max-mobile_base:pt-12 pt-16 justify-start items-center fade-in`}
    >
        <div
            class={`relative flex flex-col h-full w-full justify-start items-center`}
        >
            <div
                data-carousel-container={`keypair_new`}
                class={`carousel-container flex h-full w-full rounded-2xl scrollbar-hide`}
            >
                <div
                    data-carousel-item={`keypair_new`}
                    class={`carousel-item flex flex-col w-full max-mobile_y:pt-12 pt-12 pb-4 gap-8 justify-start items-center`}
                >
                    <div
                        class={`flex flex-col w-full gap-8 justify-start items-center`}
                    >
                        <div
                            class={`flex flex-row w-full justify-center items-center`}
                        >
                            <p
                                class={`font-mono font-[600] text-layer-0-glyph text-3xl`}
                            >
                                {`Keypair Profile`}
                            </p>
                        </div>
                    </div>
                    <div
                        class={`flex flex-col w-full gap-4 justify-start items-center`}
                    >
                        {#each keypair_new_buttons as btn}
                            <button
                                class={`flex flex-row h-16 w-${$app_layout} py-2 px-4 gap-4 justify-center items-center bg-layer-1-surface rounded-touch touch-layer-1 touch-layer-1-raise-less`}
                                on:click|preventDefault={async () => {
                                    handle_keypair_buttons_select(btn.key);
                                }}
                            >
                                <div
                                    class={`flex flex-row h-full w-12 justify-center items-center bg-layer-1-surface rounded-xl`}
                                >
                                    <Glyph
                                        basis={{
                                            classes: `text-layer-1-glyph`,
                                            key: btn.icon,
                                            dim: `lg`,
                                            weight: btn.weight || `fill`,
                                        }}
                                    />
                                </div>
                                <div
                                    class={`flex flex-col flex-grow h-full justify-between items-start`}
                                >
                                    <div class={`flex flex-row gap-[2px]`}>
                                        <p
                                            class={`font-sans text-[1.1rem] font-[500] text-layer-1-glyph/80`}
                                        >
                                            {btn.heading}
                                        </p>
                                    </div>
                                    <p
                                        class={`font-sans text-[0.9rem] text-layer-1-glyph-shade`}
                                    >
                                        {btn.subtitle}
                                    </p>
                                </div>
                                <div
                                    class={`flex flex-row h-full w-6 justify-end items-center`}
                                >
                                    <Glyph
                                        basis={{
                                            key: keypair_new_buttons_selected.includes(
                                                btn.key,
                                            )
                                                ? `check-circle`
                                                : `circle`,
                                            classes:
                                                keypair_new_buttons_selected.includes(
                                                    btn.key,
                                                )
                                                    ? `text-layer-1-glyph-hl`
                                                    : `text-layer-1-glyph-shade`,
                                            dim: `md+`,
                                            weight: keypair_new_buttons_selected.includes(
                                                btn.key,
                                            )
                                                ? `fill`
                                                : `bold`,
                                        }}
                                    />
                                </div>
                            </button>
                        {/each}
                    </div>
                </div>
            </div>
            <div
                class={`absolute bottom-0 left-0 flex flex-col w-full justify-center items-center`}
            >
                {#if keypair_new_buttons_selected.length}
                    <button
                        class={`group flex flex-row h-[3.25rem] w-${$app_layout} justify-center items-center bg-layer-1-surface rounded-touch touch-layer-1 fade-in`}
                        on:click={async () => {
                            alert(`@todo`);
                        }}
                    >
                        <p
                            class={`font-sans font-[600] tracking-wide text-layer-1-glyph/80 group-active:text-layer-1-glyph/40 transition-all`}
                        >
                            {`${$t(`common.continue`)}`}
                        </p>
                    </button>
                {:else}
                    <div
                        class={`flex flex-row h-12 w-full justify-start items-center`}
                    >
                        <Fill />
                    </div>
                {/if}
            </div>
        </div>
    </div>
</LayoutView>
