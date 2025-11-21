<script lang="ts">
    import { ls } from "$lib/utils/i18n";
    import { Settings } from "@radroots/apps-lib-pwa";
</script>

<Settings
    basis={{
        trellis_ext: [
            {
                list: [
                    {
                        hide_active: true,
                        touch: {
                            label: {
                                left: [
                                    {
                                        value: `${$ls(`common.logout`)}`,
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
                                alert("not implemented");
                            },
                        },
                    },
                ],
            },
        ],
    }}
/>
