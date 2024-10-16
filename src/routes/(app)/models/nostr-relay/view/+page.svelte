<script lang="ts">
    import { db, nostr } from "$lib/client";
    import type { NostrProfile, NostrRelay } from "@radroots/models";
    import {
        app_blur,
        app_notify,
        EnvelopeButtons,
        LayoutTrellis,
        LayoutView,
        Nav,
        nostr_relays_connected,
        qp_id,
        t,
        Trellis,
    } from "@radroots/svelte-lib";
    import { onMount } from "svelte";

    type LoadData = {
        nostr_relay: NostrRelay;
        nostr_profiles: NostrProfile[];
        nostr_profiles_unconnected: NostrProfile[];
    };
    let ld: LoadData | undefined = undefined;
    let show_edit = false;

    onMount(async () => {
        try {
            if (!$qp_id) app_notify.set(`${$t(`error.client.page.load`)}`);
            ld = await load_data();
        } catch (e) {
        } finally {
        }
    });

    const load_data = async (): Promise<LoadData | undefined> => {
        try {
            const nostr_relays = await db.nostr_relay_get({
                id: $qp_id,
            });
            if (`err` in nostr_relays) {
                app_notify.set(`${$t(`error.client.page.load`)}`);
                return;
            } else if (nostr_relays.results.length < 1) {
                app_notify.set(`${$t(`error.client.page.load`)}`);
                return;
            }

            const nostr_relay = nostr_relays.results[0];

            const nostr_profiles = await db.nostr_profile_get({
                list: [`on_relay`, { id: nostr_relay.id }],
            });

            const nostr_profiles_unconnected = await db.nostr_profile_get({
                list: [`off_relay`, { id: nostr_relay.id }],
            });

            const data: LoadData = {
                nostr_relay,
                nostr_profiles:
                    `results` in nostr_profiles ? nostr_profiles.results : [],
                nostr_profiles_unconnected:
                    `results` in nostr_profiles_unconnected
                        ? nostr_profiles_unconnected.results
                        : [],
            };
            return data;
        } catch (e) {
            console.log(`(error) load_data `, e);
        }
    };

    $: {
        app_blur.set(show_edit);
    }

    $: {
        console.log(JSON.stringify(ld, null, 4), `ld`);
    }
    const relay_connect = async (): Promise<void> => {
        try {
        } catch (e) {
            console.log(`(error) relay_connect `, e);
        }
    };

    const relay_disconnect = async (): Promise<void> => {
        try {
        } catch (e) {
            console.log(`(error) relay_disconnect `, e);
        }
    };
</script>

<LayoutView>
    <LayoutTrellis>
        {#if ld}
            <Trellis
                basis={{
                    args: {
                        layer: 1,
                        title: {
                            value: `${$t(`common.url`)}`,
                        },
                        list: [
                            {
                                hide_active: true,
                                touch: {
                                    label: {
                                        left: [
                                            {
                                                classes: `text-layer-1-glyph`,
                                                value: ld.nostr_relay.url,
                                            },
                                        ],
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
                            value: `${$t(`common.status`)}`,
                        },
                        list: [
                            {
                                hide_active: true,
                                touch: {
                                    label: {
                                        left: $nostr_relays_connected.includes(
                                            ld.nostr_relay.id,
                                        )
                                            ? [
                                                  {
                                                      glyph: {
                                                          classes: `text-success`,
                                                          key: `globe`,
                                                      },
                                                  },
                                                  {
                                                      classes: `text-success font-[500]`,
                                                      value: `${$t(`common.connected`)}`,
                                                  },
                                              ]
                                            : [
                                                  {
                                                      glyph: {
                                                          classes: `text-yellow-700/80`,
                                                          key: `globe-x`,
                                                      },
                                                  },
                                                  {
                                                      classes: `text-yellow-700/80 font-[500] capitalize`,
                                                      value: `${$t(`common.not_connected`)}`,
                                                  },
                                              ],
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
                            value: `${$t(`model.nostr_relay.pubkey`)}`,
                        },
                        list: [
                            {
                                hide_active: true,
                                touch: {
                                    label: {
                                        left: [
                                            {
                                                classes: `text-layer-1-glyph`,
                                                value: ld.nostr_relay.pubkey
                                                    ? nostr.lib.npub(
                                                          ld.nostr_relay.pubkey,
                                                          true,
                                                      )
                                                    : `${$t(`icu.no_*_published`, { value: `${$t(`pubkey`)}`.toLowerCase() })}`,
                                            },
                                        ],
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
                            value: `${$t(`model.nostr_relay.description`)}`,
                        },
                        list: [
                            {
                                hide_active: true,
                                touch: {
                                    label: {
                                        left: [
                                            {
                                                classes: `text-layer-1-glyph`,
                                                value:
                                                    ld.nostr_relay
                                                        .description ||
                                                    `${$t(`icu.no_*_published`, { value: `${$t(`model.nostr_relay.description`)}`.toLowerCase() })}`,
                                            },
                                        ],
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
                            value: `${$t(`model.nostr_relay.software`)}`,
                        },
                        list: [
                            {
                                hide_active: true,
                                touch: {
                                    label: {
                                        left: [
                                            {
                                                classes: `text-layer-1-glyph`,
                                                value:
                                                    ld.nostr_relay.software ||
                                                    `${$t(`icu.no_*_published`, { value: `${$t(`model.nostr_relay.software`)}`.toLowerCase() })}`,
                                            },
                                        ],
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
                            value: `${$t(`model.nostr_relay.version`)}`,
                        },
                        list: [
                            {
                                hide_active: true,
                                touch: {
                                    label: {
                                        left: [
                                            {
                                                classes: `text-layer-1-glyph`,
                                                value:
                                                    ld.nostr_relay.version ||
                                                    `${$t(`icu.no_*_published`, { value: `${$t(`model.nostr_relay.version`)}`.toLowerCase() })}`,
                                            },
                                        ],
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
                            value: `${$t(`model.nostr_relay.supported_nips`)}`,
                        },
                        list: [
                            {
                                hide_active: true,
                                touch: {
                                    label: {
                                        left: [
                                            {
                                                classes: `text-layer-1-glyph`,
                                                value:
                                                    ld.nostr_relay
                                                        .supported_nips ||
                                                    `${$t(`icu.no_*_published`, { value: `${$t(`supported_nips`)}`.toLowerCase() })}`,
                                            },
                                        ],
                                    },
                                },
                            },
                        ],
                    },
                }}
            />
        {/if}
    </LayoutTrellis>
</LayoutView>
<Nav
    basis={{
        prev: {
            label: `${$t(`common.back`)}`,
            route: `/models/nostr-relay`,
        },
        title: {
            label: {
                classes: `capitalize`,
                value: `${$t(`common.relay`)}`,
            },
        },
        option: {
            label: {
                swap: {
                    on: {
                        value: `${$t(`common.done`)}`,
                    },
                    off: {
                        value: `${$t(`common.edit`)}`,
                    },
                    toggle: show_edit,
                },
            },
            callback: async (el) => {
                if (el) show_edit = !el.classList.contains(`swap-active`);
            },
        },
    }}
/>
{#if ld}
    <EnvelopeButtons
        basis={{
            visible: show_edit,
            buttons: [
                {
                    classes: `text-envelopeButtonCancel text-layer-1-glyph-hl`,
                    label: `${$t(`common.cancel`)}`,
                    callback: async () => {
                        show_edit = false;
                    },
                },
                {
                    classes: $nostr_relays_connected.includes(ld.nostr_relay.id)
                        ? `text-envelopeButtonLabel text-red-400`
                        : `text-envelopeButtonLabel text-success`,
                    label: $nostr_relays_connected.includes(ld.nostr_relay.id)
                        ? `${$t(`icu.disconnect_*`, { value: `${$t(`common.relay`)}` })}`
                        : `${$t(`icu.connect_*`, { value: `${$t(`common.relay`)}` })}`,
                    callback: async () => {
                        if (!ld) return;
                        if ($nostr_relays_connected.includes(ld.nostr_relay.id))
                            await relay_disconnect();
                        else await relay_connect();
                        show_edit = false;
                    },
                },
            ],
        }}
    />
{/if}
