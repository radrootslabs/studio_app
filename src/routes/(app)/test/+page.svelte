<script lang="ts">
    import { LayoutView, Nav, t } from "@radroots/svelte-lib";
</script>

<LayoutView>test</LayoutView>

<Nav
    basis={{
        prev: {
            label: `${$t("common.home")}`,
            route: `/`,
        },
        title: {
            label: { value: `Image Upload` },
        },
    }}
/>
