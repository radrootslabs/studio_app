<script lang="ts">
    import { dialog, geol, haptics, keystore } from "$lib/client";
    import { ascii, ks } from "$lib/conf";
    import { reset_device } from "$lib/util/client";
    import {
        app_thc,
        LayoutTrellis,
        LayoutView,
        Nav,
        route,
        t,
        Trellis,
    } from "@radroots/svelte-lib";
    import { parse_color_mode } from "@radroots/theme";
</script>

<LayoutView>
    <LayoutTrellis>
        <Trellis
            basis={{
                args: {
                    layer: 1,
                    title: {
                        value: `Appearance`,
                    },
                    list: [
                        {
                            hide_active: true,
                            select: {
                                label: {
                                    left: [
                                        {
                                            value: `${$t(`common.color_mode`)}`,
                                            classes: `capitalize`,
                                        },
                                    ],
                                },
                                display: {
                                    label: {
                                        value: `${$app_thc}`,
                                        classes: `capitalize`,
                                    },
                                },
                                el: {
                                    value: $app_thc,
                                    options: [
                                        {
                                            entries: [
                                                {
                                                    value: ascii.bullet,
                                                    label: `${$t(`icu.choose_*`, { value: `${$t(`common.color_mode`)}`.toLowerCase() })}`,
                                                    disabled: true,
                                                },
                                                {
                                                    value: `light`,
                                                    label: `${$t(`common.light`)}`,
                                                },
                                                {
                                                    value: `dark`,
                                                    label: `${$t(`common.dark`)}`,
                                                },
                                            ],
                                        },
                                    ],
                                    callback: async ({ value }) => {
                                        if (value)
                                            app_thc.set(
                                                parse_color_mode(value),
                                            );
                                    },
                                },
                                end: {
                                    glyph: {
                                        key: `caret-right`,
                                    },
                                },
                            },
                        },
                    ],
                },
            }}
        />
        <Trellis
            basis={{
                args: {
                    layer: 1,
                    title: {
                        value: `Nostr Keys`,
                    },
                    list: [
                        {
                            touch: {
                                label: {
                                    left: [
                                        {
                                            value: `Nostr Key (public)`,
                                            classes: `capitalize`,
                                        },
                                    ],
                                },
                                end: {
                                    glyph: {
                                        key: `caret-right`,
                                    },
                                },
                                callback: async () => {
                                    const public_key = await keystore.get(
                                        ks.keys.nostr_publickey,
                                    );
                                    if (`err` in public_key) return;
                                    await dialog.alert(
                                        `Hi! This is your nostr public key ${public_key.result}`,
                                    );
                                },
                            },
                        },
                        {
                            touch: {
                                label: {
                                    left: [
                                        {
                                            value: `Nostr Key (secret)`,
                                            classes: `capitalize`,
                                        },
                                    ],
                                },
                                end: {
                                    glyph: {
                                        key: `caret-right`,
                                    },
                                },
                                callback: async () => {
                                    const public_key = await keystore.get(
                                        ks.keys.nostr_publickey,
                                    );
                                    if (`err` in public_key) return;
                                    const secret_key = await keystore.get(
                                        ks.keys.nostr_secretkey(
                                            public_key.result,
                                        ),
                                    );
                                    if (`err` in secret_key) return;
                                    await dialog.alert(
                                        `Hi! This is your nostr secret key ${secret_key.result}`,
                                    );
                                },
                            },
                        },
                    ],
                },
            }}
        />
        <Trellis
            basis={{
                args: {
                    layer: 1,
                    title: {
                        value: `Configuration`,
                    },
                    list: [
                        {
                            touch: {
                                label: {
                                    left: [
                                        {
                                            value: `Nostr Settings`,
                                            classes: `capitalize`,
                                        },
                                    ],
                                },
                                end: {
                                    glyph: {
                                        key: `caret-right`,
                                    },
                                },
                                callback: async () => {
                                    await route(`/settings/nostr`);
                                },
                            },
                        },
                        {
                            touch: {
                                label: {
                                    left: [
                                        {
                                            value: `Reset Device`,
                                            classes: `capitalize`,
                                        },
                                    ],
                                },
                                end: {
                                    glyph: {
                                        key: `caret-right`,
                                    },
                                },
                                callback: async () => {
                                    const confirm = await dialog.confirm(
                                        `Hi! This will delete your saved keys.`,
                                    );
                                    if (confirm === true) await reset_device();
                                },
                            },
                        },
                    ],
                },
            }}
        />
        <Trellis
            basis={{
                args: {
                    layer: 1,
                    title: {
                        value: `Location`,
                    },
                    list: [
                        {
                            touch: {
                                label: {
                                    left: [
                                        {
                                            value: `Geolocation Current`,
                                            classes: `capitalize`,
                                        },
                                    ],
                                },
                                callback: async () => {
                                    const pos = await geol.current();
                                    await dialog.alert(JSON.stringify(pos));
                                },
                            },
                        },
                    ],
                },
            }}
        />
        <Trellis
            basis={{
                args: {
                    layer: 1,
                    title: {
                        value: `Web`,
                    },
                    list: [
                        {
                            touch: {
                                label: {
                                    left: [
                                        {
                                            value: `Radroots.Org (Open Homepage)`,
                                        },
                                    ],
                                },
                                callback: async () => {
                                    //const url = `https://radroots.org`;
                                    //await browser.open(url);
                                },
                            },
                        },
                        {
                            touch: {
                                label: {
                                    left: [
                                        {
                                            value: `Radroots.Org (Share Homepage)`,
                                        },
                                    ],
                                },
                                callback: async () => {
                                    //await share.open({
                                    //    title: `Radroots Home Page`,
                                    //    text: `Find farmers around the world.`,
                                    //    url: `https://radroots.org`,
                                    //    dialog_title: `This is the dialog title`,
                                    //});
                                },
                            },
                        },
                        {
                            touch: {
                                label: {
                                    left: [
                                        {
                                            value: `Primal.Net (Open Profile)`,
                                        },
                                    ],
                                },
                                callback: async () => {
                                    //const public_key = await keystore.get(
                                    //    ks.keys.nostr_publickey,
                                    //);
                                    //const npub = nostr.lib.npub(public_key);
                                    //const url = `https://primal.net/p/${npub}`;
                                    //await browser.open(url);
                                },
                            },
                        },
                    ],
                },
            }}
        />
        <Trellis
            basis={{
                args: {
                    layer: 1,
                    title: {
                        value: `Device`,
                    },
                    list: [
                        {
                            touch: {
                                label: {
                                    left: [
                                        {
                                            value: `Device (Info)`,
                                        },
                                    ],
                                },
                                callback: async () => {
                                    //const data = await device.info();
                                    //await dialog.alert(JSON.stringify(data));
                                },
                            },
                        },
                        {
                            touch: {
                                label: {
                                    left: [
                                        {
                                            value: `Device (Battery)`,
                                        },
                                    ],
                                },
                                callback: async () => {
                                    //const data = await device.battery();
                                    //await dialog.alert(JSON.stringify(data));
                                },
                            },
                        },
                    ],
                },
            }}
        />
        <Trellis
            basis={{
                args: {
                    layer: 1,
                    title: {
                        value: `Tests`,
                    },
                    list: [
                        {
                            touch: {
                                label: {
                                    left: [
                                        {
                                            value: `Haptics Touch (less)`,
                                            classes: `capitalize`,
                                        },
                                    ],
                                },
                                end: {
                                    glyph: {
                                        key: `caret-right`,
                                    },
                                },
                                callback: async () => {
                                    await haptics.impact(`light`);
                                },
                            },
                        },
                        {
                            touch: {
                                label: {
                                    left: [
                                        {
                                            value: `Haptics Touch (more)`,
                                            classes: `capitalize`,
                                        },
                                    ],
                                },
                                end: {
                                    glyph: {
                                        key: `caret-right`,
                                    },
                                },
                                callback: async () => {
                                    await haptics.impact(`heavy`);
                                },
                            },
                        },
                        {
                            touch: {
                                label: {
                                    left: [
                                        {
                                            value: `Haptics Vibrate (500ms)`,
                                            classes: `capitalize`,
                                        },
                                    ],
                                },
                                end: {
                                    glyph: {
                                        key: `caret-right`,
                                    },
                                },
                                callback: async () => {
                                    await haptics.vibrate(500);
                                },
                            },
                        },
                    ],
                },
            }}
        />
    </LayoutTrellis>
</LayoutView>
<Nav
    basis={{
        prev: {
            label: `${$t(`common.home`)}`,
            route: `/`,
        },
        title: {
            label: {
                value: `${$t(`common.settings`)}`,
            },
        },
    }}
/>
