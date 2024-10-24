<script lang="ts">
    import { db, dialog } from "$lib/client";
    import {
        nostr_profile_form_fields,
        parse_nostr_profile_form_keys,
        type NostrProfile,
        type NostrProfileFields,
        type NostrProfileFormFields,
    } from "@radroots/models";
    import {
        app_notify,
        Fill,
        fmt_id,
        kv,
        LayoutTrellis,
        LayoutView,
        Nav,
        qp_nostr_pk,
        qp_rkey,
        route,
        route_prev,
        sleep,
        t,
        Trellis,
    } from "@radroots/svelte-lib";
    import { onDestroy, onMount } from "svelte";

    let el_input: HTMLInputElement | null = null;
    let el_input_loaded = false;

    type LoadData = {
        nostr_profile: NostrProfile;
        field_key: keyof NostrProfileFields;
    };
    let ld: LoadData | undefined = undefined;

    onMount(async () => {
        try {
            if (!$qp_rkey || !$qp_nostr_pk) {
                app_notify.set(
                    `${$t(`icu.error_loading_*`, { value: `${$t(`common.page`)}` })}`,
                );
                return;
            }

            ld = await load_page();
        } catch (e) {
        } finally {
        }
    });

    onDestroy(async () => {
        try {
            qp_rkey.set(``);
            qp_nostr_pk.set(``);
        } catch (e) {
        } finally {
        }
    });

    let val_field_valid = false;

    $: translated_field_key = ld?.field_key
        ? `${$t(`model.nostr_profile.${ld?.field_key}`, { default: ld?.field_key?.replaceAll(`_`, ` `) })}`.toLowerCase()
        : ``;

    $: if (el_input_loaded && el_input) {
        el_input.focus();
    }
    const load_page = async (): Promise<LoadData | undefined> => {
        try {
            const nostr_profiles = await db.nostr_profile_get({
                public_key: $qp_nostr_pk,
            });
            if (`err` in nostr_profiles) {
                app_notify.set(
                    `${$t(`icu.error_loading_*`, { value: `${$t(`common.profile`)}` })}`,
                );
                return;
            } else if (nostr_profiles.results.length < 1) {
                app_notify.set(
                    `${$t(`icu.error_loading_*`, { value: `${$t(`common.page`)}` })}`,
                );
                return;
            }

            const field_key = parse_nostr_profile_form_keys($qp_rkey);
            if (!field_key) {
                app_notify.set(`${$t(`error.client.page.load`)}`);
                return;
            }

            const data: LoadData = {
                nostr_profile: nostr_profiles.results[0],
                field_key,
            };
            return data;
        } catch (e) {
            console.log(`(error) load_page `, e);
        }
    };

    const submit = async (): Promise<void> => {
        try {
            if (!ld?.field_key || !ld?.nostr_profile) return;

            const val = await kv.get(fmt_id($qp_rkey));
            if (!val) {
                await route(`/models/nostr-profile`);
                return;
            }
            const validated =
                nostr_profile_form_fields[ld?.field_key].validation.test(val);
            if (!validated) {
                dialog.alert(
                    `${$t(`icu.invalid_*_entry`, { value: translated_field_key })}`,
                );
                return;
            }

            const fields: Partial<NostrProfileFormFields> = {};
            fields[ld?.field_key] = val;

            const update_res = await db.nostr_profile_update({
                on: {
                    public_key: $qp_nostr_pk,
                },
                fields,
            });
            if (`err` in update_res) {
                await dialog.alert(`${$t(`error.client.unhandled`)}`);
                return;
            }

            // @todo sync to nostr
            if (ld)
                await route_prev(`/models/nostr-profile/view`, [
                    [`id`, ld.nostr_profile.id],
                ]);
            else await route_prev(`/models/nostr-profile`);
            return;
        } catch (e) {
            console.log(`(error) submit `, e);
        }
    };
</script>

<LayoutView>
    <LayoutTrellis>
        {#if ld}
            <Trellis
                basis={{
                    args: {
                        hide_offset: true,
                        layer: 1,
                        title: {
                            value: ld?.nostr_profile[ld?.field_key]
                                ? `${$t(`icu.edit_*`, { value: translated_field_key })}`
                                : `${$t(`icu.add_a_*`, { value: translated_field_key.toLowerCase() })}`,
                        },
                        list: [
                            {
                                hide_active: true,
                                input: {
                                    basis: {
                                        id: fmt_id($qp_rkey),
                                        sync: true,
                                        sync_init: ld?.nostr_profile[
                                            ld?.field_key
                                        ]
                                            ? ld.nostr_profile[ld.field_key]
                                            : true,
                                        classes: `placeholder:font-[300]`,
                                        placeholder: ld?.nostr_profile[
                                            ld?.field_key
                                        ]
                                            ? `${$t(`icu.enter_new_*`, { value: translated_field_key.toLowerCase() })}`
                                            : `${$t(`icu.add_a_*`, { value: translated_field_key.toLowerCase() })}`,
                                        field: {
                                            charset:
                                                nostr_profile_form_fields[
                                                    ld?.field_key
                                                ].charset,
                                            validate:
                                                nostr_profile_form_fields[
                                                    ld?.field_key
                                                ].validation,
                                            validate_keypress: false,
                                        },
                                        callback: async ({ pass }) => {
                                            val_field_valid = pass;
                                        },
                                        callback_keydown: async ({ key }) => {
                                            if (key === `Enter`) await submit();
                                        },
                                        on_mount: async (el) => {
                                            el_input = el;
                                            await sleep(600); //@todo
                                            el_input_loaded = true;
                                        },
                                    },
                                },
                            },
                        ],
                    },
                }}
            >
                <div
                    slot="offset"
                    class={`flex flex-row w-4 justify-start items-center`}
                >
                    <Fill />
                </div>
            </Trellis>
        {/if}
    </LayoutTrellis>
</LayoutView>
<Nav
    basis={{
        prev: {
            label: `${$t(`common.back`)}`,
            route: `/models/nostr-profile`,
        },
        title: {
            label: {
                classes: `capitalize`,
                value: translated_field_key,
            },
        },
        option: {
            label: {
                classes: val_field_valid ? `` : `opacity-60`,
                value: ld?.nostr_profile[ld?.field_key]
                    ? `${$t(`common.update`)}`
                    : `${$t(`common.add`)}`,
            },
            callback: async () => {
                if (val_field_valid) await submit();
            },
        },
    }}
/>
