<script lang="ts">
    import { db, dialog } from "$lib/client";
    import { nostr_sync_metadata } from "$lib/util/nostr-sync";
    import {
        nostr_profile_form_fields,
        type NostrProfile,
        type NostrProfileFields,
        type NostrProfileFormFields,
        parse_nostr_profile_form_keys,
    } from "@radroots/models";
    import {
        app_notify,
        catch_err,
        fmt_id,
        InputElement,
        kv,
        LayoutView,
        ls,
        Nav,
        qp_nostr_pk,
        qp_rkey,
        route,
        TextareaElement,
    } from "@radroots/svelte-lib";
    import { onDestroy, onMount } from "svelte";

    type LoadData = {
        nostr_profile: NostrProfile;
        field_key: keyof NostrProfileFields;
    };
    let ld: LoadData | undefined = undefined;

    let page_initial_value = ``;
    let page_input_value = ``;

    onMount(async () => {
        try {
            console.log(`$qp_rkey `, $qp_rkey);
            console.log(`$qp_nostr_pk `, $qp_nostr_pk);
            if (!$qp_rkey || !$qp_nostr_pk) {
                app_notify.set(
                    `${$ls(`icu.error_loading_*`, { value: `${$ls(`common.page`)}` })}`,
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
            await nostr_sync_metadata();
        } catch (e) {
        } finally {
        }
    });

    $: translated_field_key = ld?.field_key
        ? `${$ls(`models.nostr_profile.fields.${ld.field_key}.label`)}`
        : ``;
    $: input_value_del = page_initial_value !== page_input_value;

    const load_page = async (): Promise<LoadData | undefined> => {
        try {
            const nostr_profile = await db.nostr_profile_get_one({
                public_key: $qp_nostr_pk,
            });
            if (`err` in nostr_profile) {
                app_notify.set(
                    `${$ls(`icu.error_loading_*`, { value: `${$ls(`common.profile`)}` })}`,
                );
                return;
            }
            const field_key = parse_nostr_profile_form_keys($qp_rkey);
            if (!field_key) {
                app_notify.set(`${$ls(`error.client.page.load`)}`);
                return;
            }
            const field_val = nostr_profile.result[field_key];
            if (field_val) {
                page_input_value = field_val;
                page_initial_value = field_val;
            }
            return {
                nostr_profile: nostr_profile.result,
                field_key,
            } satisfies LoadData;
        } catch (e) {
            await catch_err(e, `load_page`);
        }
    };

    const submit = async (): Promise<void> => {
        try {
            if (!ld?.field_key || !ld?.nostr_profile) return;
            const val = await kv.get(fmt_id($qp_rkey));
            if (!val) {
                await route(`/settings/profile`);
                return;
            }
            const validated =
                nostr_profile_form_fields[ld.field_key].validation.test(val);
            if (!validated) {
                dialog.alert(
                    `${$ls(`icu.invalid_*_entry`, { value: translated_field_key })}`,
                );
                return;
            }
            const fields: Partial<NostrProfileFormFields> = {};
            fields[ld.field_key] = val;
            const update_res = await db.nostr_profile_update({
                on: {
                    public_key: $qp_nostr_pk,
                },
                fields,
            });
            if (`err` in update_res) {
                await dialog.alert(`${$ls(`error.client.unhandled`)}`);
                return;
            }
            await route(`/settings/profile`);
        } catch (e) {
            await catch_err(e, `submit`);
        }
    };
</script>

<LayoutView>
    {#if ld}
        <div
            class={`flex flex-col w-full pt-4 px-4 gap-1 justify-start items-center fade-in`}
        >
            <div class={`flex flex-row w-full justify-start items-center`}>
                <p
                    class={`font-sans font-[400] text-[0.8rem] text-layer-0-glyph-label uppercase`}
                >
                    {translated_field_key.replace(`Profile `, ``)}
                </p>
            </div>
            {#if ld.field_key === `about`}
                <TextareaElement
                    bind:value={page_input_value}
                    basis={{
                        id: fmt_id(ld.field_key),
                        classes: `min-h-[8rem] pl-4`,
                        sync: true,
                        layer: 1,
                        placeholder: `Enter ${translated_field_key.toLowerCase()}`,
                        field: {
                            charset:
                                nostr_profile_form_fields[ld.field_key].charset,
                            validate:
                                nostr_profile_form_fields[ld.field_key]
                                    .validation,
                            validate_keypress: true,
                        },
                        callback_focus: async () => {
                            console.log(`hi`);
                        },
                    }}
                />
            {:else}
                <InputElement
                    bind:value={page_input_value}
                    basis={{
                        id: fmt_id(ld.field_key),
                        classes: `rounded-touch pl-4`,
                        sync: true,
                        layer: 1,
                        placeholder: `Enter ${translated_field_key.toLowerCase()}`,
                        field: {
                            charset:
                                nostr_profile_form_fields[ld.field_key].charset,
                            validate:
                                nostr_profile_form_fields[ld.field_key]
                                    .validation,
                            validate_keypress: true,
                        },
                        callback_focus: async () => {
                            console.log(`hi`);
                        },
                    }}
                />
            {/if}
        </div>
    {/if}
</LayoutView>
<Nav
    basis={{
        prev: {
            label: `${$ls(`common.back`)}`,
            route: `/settings/profile`,
            prevent_route: input_value_del
                ? {
                      callback: async () => {
                          if (input_value_del) await submit();
                      },
                  }
                : undefined,
        },
        title: {
            label: {
                classes: `capitalize`,
                value: `${$ls(`icu.edit_*`, { value: `${$ls(`common.profile`)}` })}`,
            },
        },
        /*option: {
            label: {
                classes: input_value_del ? `` : `opacity-60`,
                value: ld?.nostr_profile[ld?.field_key]
                    ? `${$ls(`common.update`)}`
                    : `${$ls(`common.add`)}`,
            },
            callback: async () => {
                if (input_value_del) await submit();
            },
        },*/
    }}
/>
