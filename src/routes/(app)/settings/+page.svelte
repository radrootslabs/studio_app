<script lang="ts">
    import { dialog, geol, haptics, keystore } from "$lib/client";
    import { ks } from "$lib/conf";
    import {
        app_notify,
        app_thc,
        LayoutTrellis,
        LayoutView,
        Nav,
        t,
        toggle_color_mode,
        Trellis,
    } from "@radroots/svelte-lib";
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
                            touch: {
                                label: {
                                    left: [
                                        {
                                            value: `Toggle Color Mode (${toggle_color_mode($app_thc)})`,
                                            classes: `capitalize`,
                                        },
                                    ],
                                },
                                callback: async () => {
                                    await haptics.impact();
                                    app_thc.set(toggle_color_mode($app_thc));
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
                                    icon: {
                                        key: `caret-right`,
                                    },
                                },
                                callback: async () => {
                                    const public_key = await keystore.get(
                                        ks.nostr.nostr_key_active,
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
                                    icon: {
                                        key: `caret-right`,
                                    },
                                },
                                callback: async () => {
                                    const public_key = await keystore.get(
                                        ks.nostr.nostr_key_active,
                                    );
                                    if (`err` in public_key) return;
                                    const secret_key = await keystore.get(
                                        ks.nostr.nostr_key(public_key.result),
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
                                            value: `Reset Device`,
                                            classes: `capitalize`,
                                        },
                                    ],
                                },
                                end: {
                                    icon: {
                                        key: `caret-right`,
                                    },
                                },
                                callback: async () => {
                                    const confirm = await dialog.confirm(
                                        `Hi! This will delete your saved keys.`,
                                    );
                                    if (confirm) {
                                        const ks_nostr_pk = await keystore.get(
                                            ks.nostr.nostr_key_active,
                                        );
                                        if (`err` in ks_nostr_pk) {
                                            await dialog.alert(
                                                `There was an error.`,
                                            );
                                            return;
                                        }
                                        await keystore.remove(
                                            ks.nostr.nostr_key(
                                                ks_nostr_pk.result,
                                            ),
                                        );
                                        await keystore.remove(
                                            ks.nostr.nostr_key_active,
                                        );
                                        app_notify.set(
                                            `Your device has been reset.`,
                                        );
                                    }
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
                                    //    ks.nostr.nostr_key_active,
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
                                    icon: {
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
                                    icon: {
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
                                    icon: {
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
            label: `Home`,
            route: `/`,
        },
        title: {
            label: {
                value: `${$t(`common.settings`)}`,
            },
        },
    }}
/>
