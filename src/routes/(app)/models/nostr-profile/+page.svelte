<script lang="ts">
    import { db, keystore } from "$lib/client";
    import {
        nostr_profile_form_vals,
        parse_nostr_profile_form_keys,
        type NostrProfile,
    } from "@radroots/models";
    import {
        app_notify,
        as_glyph_key,
        LayoutTrellis,
        LayoutView,
        Nav,
        t,
        Trellis,
    } from "@radroots/svelte-lib";
    import { onMount } from "svelte";

    type LoadData = {
        nostr_profiles: NostrProfile[];
    };
    let ld: LoadData | undefined = undefined;

    onMount(async () => {
        try {
            ld = await load_data();
        } catch (e) {
        } finally {
        }
    });

    $: {
        console.log(JSON.stringify(ld, null, 4), `ld`);
    }

    const load_data = async (): Promise<LoadData | undefined> => {
        try {
            const nostr_profiles = await db.nostr_profile_get({
                list: [`all`],
            });
            if (`err` in nostr_profiles) {
                app_notify.set(`${$t(`error.client.page.load`)}`);
                return;
            }
            const data: LoadData = {
                nostr_profiles: nostr_profiles.results,
            };
            return data;
        } catch (e) {
            console.log(`(error) load_data `, e);
        }
    };

    const handle_add_location_gcs = async (): Promise<void> => {
        try {
            console.log(`@todo`);
            //const res = await location_gcs_add_current();
            //if (res) ld = await load_data();
        } catch (e) {
            console.log(`(error) handle_add_location_gcs `, e);
        }
    };

    const handle_add_nostr_profile = async (): Promise<void> => {
        try {
            const ks_keys = await keystore.keys();
            console.log(JSON.stringify(ks_keys, null, 4), `ks_keys`);
        } catch (e) {
            console.log(`(error) handle_add_nostr_profile `, e);
        }
    };
</script>

<LayoutView>
    <LayoutTrellis>
        {#if ld}
            {#each ld.nostr_profiles as li}
                <Trellis
                    basis={{
                        args: {
                            layer: 1,
                            title: {
                                value: `${$t(`icu.your_*`, { value: `${$t(`common.profiles`)}` })}`,
                            },
                            list: [
                                ...Object.keys(nostr_profile_form_vals).map(
                                    (k) => ({
                                        hide_active: true,
                                        touch: {
                                            label: {
                                                left: [
                                                    {
                                                        classes: `capitalize`,
                                                        value: `${$t(`model.nostr_profile.${k}`, { default: k.replaceAll(`_`, ` `) })}`,
                                                    },
                                                ],
                                                right: [
                                                    {
                                                        classes: `font-[300] text-layer-1-glyph-shade`,
                                                        value:
                                                            Object.assign(li)[
                                                                parse_nostr_profile_form_keys(
                                                                    k,
                                                                )
                                                            ] || "(none)",
                                                    },
                                                ],
                                            },
                                            end: {
                                                icon: {
                                                    key: as_glyph_key(
                                                        `caret-right`,
                                                    ),
                                                },
                                            },
                                            callback: async () => {},
                                        },
                                    }),
                                ),
                            ],
                        },
                    }}
                />
            {/each}
        {:else}
            <div
                class={`flex flex-col w-full justify-center items-center px-4 gap-3`}
            >
                <p class={`font-sans font-[400] text-layer-2-glyph`}>
                    {`No items to display.`}
                </p>

                <button
                    class={`flex flex-row justify-center items-center`}
                    on:click={async () => {
                        await handle_add_location_gcs();
                    }}
                >
                    <p
                        class={`font-sans font-[400] text-layer-2-glyph-hl text-sm`}
                    >
                        {`Click to add a new location`}
                    </p>
                </button>
            </div>
        {/if}
    </LayoutTrellis>
</LayoutView>
<Nav
    basis={{
        prev: {
            label: `${$t(`common.back`)}`,
            route: `/`,
        },
        title: {
            label: {
                value: `${$t(`common.profiles`)}`,
            },
        },
        option: ld?.nostr_profiles?.length
            ? {
                  label: {
                      value: `${$t(`common.add`)}`,
                      classes: `tap-color`,
                  },
                  callback: async () => {
                      await handle_add_nostr_profile();
                  },
              }
            : undefined,
    }}
/>

<style>
    :global(.map-card) {
        height: 100px;
        width: 160px;
    }
</style>
