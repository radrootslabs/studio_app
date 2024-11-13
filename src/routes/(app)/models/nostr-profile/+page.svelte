<script lang="ts">
    import { db, keystore, nostr } from "$lib/client";
    import { type NostrProfile } from "@radroots/models";
    import {
        app_nostr_key,
        app_notify,
        Glyph,
        type ISelectOption,
        LayoutTrellis,
        LayoutView,
        Nav,
        nav_prev,
        route,
        SelectMenu,
        t,
        TrellisTitle,
    } from "@radroots/svelte-lib";
    import { onMount } from "svelte";

    type OptionsListKey =
        | `view-key`
        | `set-key-active`
        | `edit-profile-name`
        | `add-profile-name`
        | `delete-key`;
    const page_param: {
        options_list: ISelectOption<OptionsListKey>[];
    } = {
        options_list: [
            {
                value: `view-key`,
                label: `${$t(`icu.view_*`, { value: `${$t(`common.details`)}`.toLowerCase() })}`,
            },
            {
                value: `edit-profile-name`,
                label: `${$t(`icu.edit_*`, { value: `${$t(`common.profile_name`)}`.toLowerCase() })}`,
            },
            {
                value: `add-profile-name`,
                label: `${$t(`icu.add_*`, { value: `${$t(`common.profile_name`)}`.toLowerCase() })}`,
            },
            {
                value: `set-key-active`,
                label: `${$t(`icu.set_as_*`, { value: `${$t(`common.active`)}`.toLowerCase() })}`,
            },
            {
                value: `delete-key`,
                label: `${$t(`icu.delete_*`, { value: `${$t(`common.key`)}`.toLowerCase() })}`,
            },
        ],
    };

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
            console.log(
                JSON.stringify(nostr_profiles, null, 4),
                `nostr_profiles`,
            );
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
            const ks_keys = await keystore.entries();
            console.log(JSON.stringify(ks_keys, null, 4), `ks_keys`);
        } catch (e) {
            console.log(`(error) handle_add_nostr_profile `, e);
        }
    };

    const handle_key_options_press = async (opts: {
        option: string;
        public_key: string;
    }): Promise<void> => {
        switch (opts.option) {
            case `view-key`:
            case `add-profile-name`:
            case `edit-profile-name`: {
                $nav_prev.push({
                    route: `/models/nostr-profile/view`,
                    label: `Keys`,
                });
            }

            case `view-key`:
                {
                    await route(`/models/nostr-profile/view`, [
                        [`nostr_pk`, opts.public_key],
                    ]);
                }
                break;
            case `add-profile-name`:
            case `edit-profile-name`:
                {
                    await route(`/models/nostr-profile/edit/field`, [
                        [`nostr_pk`, opts.public_key],
                        [`rkey`, `name`],
                    ]);
                }
                break;
        }
    };
</script>

<LayoutView>
    {#if ld}
        <LayoutTrellis>
            <div
                class={`flex flex-col w-full gap-[2px] justify-start items-center`}
            >
                <TrellisTitle
                    layer={0}
                    basis={{
                        value: `${$t(`icu.*_list`, { value: `${$t(`common.profile`)}`.toLowerCase() })}`,
                    }}
                />
                {#if ld.nostr_profiles.length}
                    <div
                        class={`flex flex-col w-full gap-4 justify-center items-center`}
                    >
                        {#each ld.nostr_profiles as li (li.public_key)}
                            <div
                                class={`relative flex flex-col h-24 pt-5 px-3 bg-layer-1-surface rounded-touch overflow-hidden active:ring-4 active:ring-layer-2-surface/80 transition-all tap-rise-1 active:opacity-60`}
                            >
                                <button
                                    class={`flex flex-col h-full w-full pt-[2px] pl-1 gap-1 items-start`}
                                    on:click|preventDefault={async () => {
                                        $nav_prev.push({
                                            route: `/models/nostr-profile`,
                                            label: `${$t(`common.profiles`)}`,
                                        });
                                        await route(
                                            `/models/nostr-profile/view`,
                                            [[`nostr_pk`, li.public_key]],
                                        );
                                    }}
                                >
                                    <div
                                        class={`flex flex-row w-full pl-1 gap-4 justify-start items-center`}
                                    >
                                        <p
                                            class={`font-mono text-[1.1rem] text-layer-1-glyph-shade text-ellipsis overflow-hidden`}
                                        >
                                            {li.name
                                                ? li.name
                                                : `(${`${$t(`icu.no_*`, { value: `${$t(`common.profile`)}` })}`})`}
                                        </p>
                                        {#if li.public_key === $app_nostr_key}
                                            <div class={`flex flex-row`}>
                                                <div
                                                    class={`flex flex-row h-4 justify-center items-center px-[6px] bg-success/70 rounded-md -translate-y-[1px]`}
                                                >
                                                    <p
                                                        class={`font-mono font-[900] text-[0.7rem] text-white text-ellipsis overflow-hidden`}
                                                    >
                                                        {`${$t(`common.active`)}`}
                                                    </p>
                                                </div>
                                            </div>
                                        {/if}
                                    </div>
                                    <div
                                        class={`grid grid-cols-12 flex flex-row h-6 w-full pt-2 gap-2 items-center`}
                                    >
                                        <div
                                            class={`col-span-2 flex flex-row h-full items-center `}
                                        >
                                            <div
                                                class={`flex flex-row h-[1rem] px-[9px] justify-start items-center bg-zinc-800/90 rounded-[5px] translate-y-[1px]`}
                                            >
                                                <p
                                                    class={`font-mono font-[600] text-[0.9rem] text-layer-2-glyph lowercase line-clamp-1`}
                                                >
                                                    {`${$t(`common.key`)}`}
                                                </p>
                                            </div>
                                        </div>
                                        <div
                                            class={`col-span-10 flex flex-row h-full pr-2 justify-end items-center overflow-x-hidden`}
                                        >
                                            <p
                                                class={`font-mono text-[0.9rem] text-layer-1-glyph line-clamp-1`}
                                            >
                                                {`${`${nostr.lib.npub(li.public_key) || ""}`.slice(
                                                    0,
                                                    24,
                                                )}...`}
                                            </p>
                                        </div>
                                    </div>
                                </button>
                                <div
                                    class={`z-10 absolute top-2 right-3 flex flex-row h-full justify-end pr-1`}
                                >
                                    <SelectMenu
                                        value={`~`}
                                        basis={{
                                            layer: 0,
                                            //mask: true,
                                            callback: async ({ value }) => {
                                                await handle_key_options_press({
                                                    option: value,
                                                    public_key: li.public_key,
                                                });
                                            },
                                            options: [
                                                {
                                                    entries:
                                                        page_param.options_list.filter(
                                                            (i) =>
                                                                !(
                                                                    !li.name &&
                                                                    i.value ===
                                                                        `edit-profile-name`
                                                                ) &&
                                                                !(
                                                                    li.name &&
                                                                    i.value ===
                                                                        `add-profile-name`
                                                                ) &&
                                                                !(
                                                                    li.public_key ===
                                                                        $app_nostr_key &&
                                                                    i.value ===
                                                                        `set-key-active`
                                                                ),
                                                        ),
                                                },
                                            ],
                                        }}
                                    >
                                        <svelte:fragment slot="element">
                                            <Glyph
                                                basis={{
                                                    key: `dots-three`,
                                                    dim: `md`,
                                                    classes: `text-layer-1-glyph`,
                                                    weight: `bold`,
                                                }}
                                            />
                                        </svelte:fragment>
                                    </SelectMenu>
                                </div>
                            </div>
                        {/each}
                    </div>
                {/if}
            </div>
        </LayoutTrellis>
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
                <p class={`font-sans font-[400] text-layer-2-glyph-hl text-sm`}>
                    {`Click to add a new location`}
                </p>
            </button>
        </div>
    {/if}
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
