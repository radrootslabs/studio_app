<script lang="ts">
    import { db, nostr } from "$lib/client";
    import type { NostrProfile, NostrRelay } from "@radroots/models";
    import {
        app_blur,
        app_notify,
        catch_err,
        EnvelopeButtons,
        LayoutTrellis,
        LayoutView,
        ls,
        Nav,
        nostr_relays_connected,
        qp_id,
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
            if (!$qp_id) app_notify.set(`${$ls(`error.client.page.load`)}`);
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
                app_notify.set(`${$ls(`error.client.page.load`)}`);
                return;
            } else if (!nostr_relays.results.length) {
                app_notify.set(`${$ls(`error.client.page.load`)}`);
                return;
            }
            const nostr_relay = nostr_relays.results[0];
            const nostr_profiles = await db.nostr_profile_get({
                list: [`on_relay`, { id: nostr_relay.id }],
            });
            const nostr_profiles_unconnected = await db.nostr_profile_get({
                list: [`off_relay`, { id: nostr_relay.id }],
            });
            return {
                nostr_relay,
                nostr_profiles:
                    `results` in nostr_profiles ? nostr_profiles.results : [],
                nostr_profiles_unconnected:
                    `results` in nostr_profiles_unconnected
                        ? nostr_profiles_unconnected.results
                        : [],
            } satisfies LoadData;
        } catch (e) {
            await catch_err(e, `load_data`);
        }
    };

    $: app_blur.set(show_edit);

    const relay_connect = async (): Promise<void> => {
        try {
        } catch (e) {
            await catch_err(e, `relay_connect`);
        }
    };

    const relay_disconnect = async (): Promise<void> => {
        try {
        } catch (e) {
            await catch_err(e, `relay_disconnect`);
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
                            value: `${$ls(`common.url`)}`,
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
                            value: `${$ls(`common.status`)}`,
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
                                                      value: `${$ls(`common.connected`)}`,
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
                                                      value: `${$ls(`common.not_connected`)}`,
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
                            value: `${$ls(`models.nostr_relay.fields.pubkey.label`)}`,
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
                                                    : `${$ls(`icu.no_*_published`, { value: `${$ls(`pubkey`)}`.toLowerCase() })}`,
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
                            value: `${$ls(`models.nostr_relay.fields.description.label`)}`,
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
                                                    `${$ls(`icu.no_*_published`, { value: `${$ls(`models.nostr_relay.fields.description.label`)}`.toLowerCase() })}`,
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
                            value: `${$ls(`models.nostr_relay.fields.software.label`)}`,
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
                                                    `${$ls(`icu.no_*_published`, { value: `${$ls(`models.fields.nostr_relay.software.label`)}`.toLowerCase() })}`,
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
                            value: `${$ls(`models.nostr_relay.fields.version.label`)}`,
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
                                                    `${$ls(`icu.no_*_published`, { value: `${$ls(`models.nostr_relay.fields.version.label`)}`.toLowerCase() })}`,
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
                            value: `${$ls(`models.nostr_relay.fields.supported_nips.label`)}`,
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
                                                    `${$ls(`icu.no_*_published`, { value: `${$ls(`models.nostr_relay.fields.supported_nips.label`)}`.toLowerCase() })}`,
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
            label: `${$ls(`common.back`)}`,
            route: `/models/nostr-relay`,
        },
        title: {
            label: {
                classes: `capitalize`,
                value: `${$ls(`common.relay`)}`,
            },
        },
        option: {
            label: {
                swap: {
                    on: {
                        value: `${$ls(`common.done`)}`,
                    },
                    off: {
                        value: `${$ls(`common.edit`)}`,
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
                    label: `${$ls(`common.cancel`)}`,
                    callback: async () => {
                        show_edit = false;
                    },
                },
                {
                    classes: $nostr_relays_connected.includes(ld.nostr_relay.id)
                        ? `text-envelopeButtonLabel text-red-400`
                        : `text-envelopeButtonLabel text-success`,
                    label: $nostr_relays_connected.includes(ld.nostr_relay.id)
                        ? `${$ls(`icu.disconnect_*`, { value: `${$ls(`common.relay`)}` })}`
                        : `${$ls(`icu.connect_*`, { value: `${$ls(`common.relay`)}` })}`,
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
