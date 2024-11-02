<script lang="ts">
    import { db, dialog, notification } from "$lib/client";
    import {
        app_cfg_type,
        app_layout,
        app_nostr_key,
        envelope_visible,
        EnvelopeLower,
        Glyph,
        LayoutView,
        nav_prev,
        route,
        t,
        Tabs,
        TrellisTitle,
        type AppConfigType,
        type CallbackPromise,
        type GlyphKey,
        type GlyphWeight,
        type NavigationRoute,
    } from "@radroots/svelte-lib";
    import { onMount } from "svelte";

    type PageParamButtons = {
        route?: NavigationRoute;
        label: string;
        key: GlyphKey;
        weight?: GlyphWeight;
        callback?: CallbackPromise;
    };
    const page_param: {
        buttons: Record<AppConfigType, PageParamButtons[]>;
    } = {
        buttons: {
            personal: [
                {
                    route: `/`,
                    label: `Marketplace`,
                    key: `basket`,
                },
                {
                    route: `/`,
                    label: `Forum`,
                    key: `basket`,
                },
            ],
            farmer: [
                {
                    route: `/`,
                    label: `Post New`,
                    key: `note-blank`,
                },
                {
                    label: `Farm Products`,
                    key: `basket`,
                    callback: async () => {
                        const trade_products = await db.trade_product_get({
                            list: [`all`],
                        });
                        if (`results` in trade_products) {
                            if (trade_products.results.length === 0) {
                                $nav_prev.push({
                                    route: `/`,
                                    label: `Back`,
                                });
                                await route(`/models/trade-product/add`);
                                return;
                            }
                        }
                        await route(`/models/trade-product`);
                    },
                },
                {
                    route: `/`,
                    label: `Cooperatives`,
                    key: `users-three`,
                },
            ],
        },
    };

    let tmp_show_no_profile = false;

    onMount(async () => {
        try {
            nav_prev.set([]);

            const nostr_profile = await db.nostr_profile_get({
                public_key: $app_nostr_key,
            });
            if (`err` in nostr_profile) {
                await dialog.alert(`@todo Nostr profile configuration failure`);
                return;
            }
            if (!nostr_profile.results[0].name) tmp_show_no_profile = true;
        } catch (e) {
        } finally {
        }
    });
</script>

<LayoutView basis={{ classes: `gap-4` }}>
    <div class={`flex flex-row w-full justify-start items-center`}>
        <button
            class={`flex flex-row pt-4 px-6 gap-2 justify-center items-center`}
            on:click={async () => {
                await notification.send(`hi`);
            }}
        >
            <p
                class={`font-mono font-[600] text-layer-2-glyph max-sm:text-lg text-xl tracking-wide`}
            >
                {`radroots (alpha-1.0.0)`}
            </p>
        </button>
    </div>
    <div class={`flex flex-col pt-2 justify-center items-center`}>
        {#if tmp_show_no_profile}
            <button
                class={`relative flex flex-row h-24 w-${$app_layout} p-4 gap-4 justify-center items-center bg-layer-2-surface/60 round-20 touch-layer-1 touch-layer-1-raise-less el-re`}
                on:click={async () => {
                    await route(`/models/nostr-profile`);
                }}
            >
                <div
                    class={`absolute left-6 flex flex-row h-full justify-end items-center`}
                >
                    <Glyph
                        basis={{
                            classes: `text-layer-2-glyph`,
                            key: `user-circle-plus`,
                            dim: `xl`,
                            weight: `bold`,
                        }}
                    />
                </div>
                <div class={`flex flex-col justify-center items-center`}>
                    <p
                        class={`font-circ font-[500] text-lg text-layer-2-glyph/90`}
                    >
                        {`No farm profile added.`}
                    </p>
                    <p
                        class={`font-circ font-[500] text-sm text-layer-2-glyph/90`}
                    >
                        {`Click to add your details`}
                    </p>
                </div>
                <Glyph
                    basis={{
                        classes: `absolute top-2 right-3 text-layer-2-glyph/40`,
                        key: `x`,
                        dim: `sm`,
                        weight: `bold`,
                    }}
                />
            </button>
        {/if}
    </div>
    <div class={`flex flex-col w-full gap-[2px] justify-start items-center`}>
        <div class={`flex flex-row w-full px-6 justify-center items-center`}>
            <TrellisTitle
                basis={{
                    value: `${$t(`common.options`)}`,
                }}
            />
        </div>
        <div class={`flex flex-col w-full gap-5 justify-start items-center`}>
            {#each page_param.buttons[$app_cfg_type] as btn}
                <button
                    class={`flex flex-row h-20 w-${$app_layout} py-2 px-6 justify-between items-center bg-layer-1-surface touch-layer-1 touch-layer-1-raise-less round-20 el-re`}
                    on:click={async () => {
                        if (btn.callback) await btn.callback();
                        else if (btn.route) await route(btn.route);
                    }}
                >
                    <div
                        class={`flex flex-row gap-4 justify-start items-center`}
                    >
                        <Glyph
                            basis={{
                                classes: `text-layer-2-glyph`,
                                key: btn.key,
                                dim: `md`,
                                weight: btn.weight || `bold`,
                            }}
                        />
                        <div class={`flex flex-row justify-start items-center`}>
                            <p
                                class={`font-mono font-[500] text-lg text-layer-2-glyph`}
                            >
                                {btn.label}
                            </p>
                        </div>
                    </div>
                    <div class={`flex flex-row justify-start items-center`}>
                        <Glyph
                            basis={{
                                key: `caret-right`,
                                dim: `sm-`,
                                weight: `bold`,
                                classes: `text-layer-2-glyph`,
                            }}
                        />
                    </div>
                </button>
            {/each}
        </div>
    </div>
</LayoutView>
<Tabs
    basis={{
        list: [
            {
                icon: `house-line`,
                label: `Home`,
                callback: async () => {
                    await route(`/`);
                    const res = await db.nostr_relay_get({ list: [`all`] });
                    console.log(JSON.stringify(res, null, 4), `res`);
                },
            },
            {
                icon: `arrows-down-up`,
                label: `Transactions`,
                callback: async () => {
                    const res = await db.nostr_relay_get({
                        list: [`on_profile`, { public_key: $app_nostr_key }],
                    });
                    console.log(JSON.stringify(res, null, 4), `res`);
                },
            },
            {
                icon: `cardholder`,
                label: `Wallet`,
                callback: async () => {
                    await route(`/test`);
                },
            },
            {
                icon: `squares-four`,
                label: `Menu`,
                callback: async () => {},
            },
        ],
    }}
/>
<EnvelopeLower
    basis={{
        close: async () => {
            envelope_visible.set(false);
        },
    }}
>
    <div class={`flex flex-col h-full w-full justify-center items-center px-2`}>
        <p class={`font-apercu font-[400] text-layer-2-glyph break-all`}>
            {`Your nostr key is ${$app_nostr_key}`}
        </p>
    </div>
</EnvelopeLower>
