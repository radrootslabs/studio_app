<script lang="ts">
    import { dialog } from "$lib/client";
    import { ascii } from "$lib/conf";
    import {
        LayoutTrellis,
        LayoutView,
        Nav,
        Trellis,
        ls,
        type ISelectOption,
    } from "@radroots/svelte-lib";
    import { onMount } from "svelte";

    const page_param: {
        select_options: ISelectOption<string>[];
    } = {
        select_options: [
            {
                value: ascii.bullet,
                label: `${$ls(`icu.choose_*`, { value: `${$ls(`common.photo_hosting`)}`.toLowerCase() })}`,
                disabled: true,
            },
            {
                value: `^radroots`,
                label: `https://radroots.org`,
            },
            {
                value: `*add`,
                label: `${$ls(`icu.add_*`, { value: `${$ls(`common.upload_url`)}`.toLowerCase() })}`,
            },
            {
                value: `*disable`,
                label: `Disable uploads`,
            },
        ],
    };

    let nostr_photohosting_sel_val = ``;
    let nostr_photohosting_sel_label = ``;

    onMount(async () => {
        try {
            await init_page();
        } catch (e) {
        } finally {
        }
    });

    const init_page = async (): Promise<void> => {
        try {
            nostr_photohosting_sel_val = `^radroots`; //@todo
        } catch (e) {
            console.log(`(error) init_page `, e);
        }
    };

    $: nostr_photohosting_sel_label = nostr_photohosting_sel_val
        ? page_param.select_options.filter(
              (i) => i.value === nostr_photohosting_sel_val,
          )?.[0].label
        : ``;

    const handle_select_option = async (
        option_value: string,
    ): Promise<void> => {
        try {
            if (!option_value.startsWith(`*`)) {
                nostr_photohosting_sel_val = option_value;
                return;
            }
            nostr_photohosting_sel_val = ``;

            await dialog.alert(`@todo`); //@todo
            nostr_photohosting_sel_val = `^radroots`;
        } catch (e) {
            console.log(`(error) handle_select_option `, e);
        }
    };
</script>

<LayoutView>
    <LayoutTrellis>
        <Trellis
            basis={{
                args: {
                    layer: 1,
                    list: [
                        {
                            hide_active: true,
                            select: {
                                label: {
                                    left: [
                                        {
                                            value: `Photo Hosting`,
                                            classes: `capitalize`,
                                        },
                                    ],
                                },
                                display: {
                                    loading: !nostr_photohosting_sel_val,
                                    label: {
                                        value: nostr_photohosting_sel_label,
                                    },
                                },
                                el: {
                                    value: nostr_photohosting_sel_val,
                                    options: [
                                        {
                                            entries: page_param.select_options,
                                        },
                                    ],
                                    callback: async ({ value }) => {
                                        await handle_select_option(value);
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
    </LayoutTrellis>
</LayoutView>
<Nav
    basis={{
        prev: {
            label: `${$ls(`common.settings`)}`,
            route: `/settings`,
        },
        title: {
            label: {
                value: `${$ls(`common.nostr`)}`,
            },
        },
    }}
/>
