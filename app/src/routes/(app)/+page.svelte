<script lang="ts">
    import { ls } from "$lib/locale/i18n";
    import { handle_err, Home, type IHomeViewData } from "@radroots/lib-app";

    let data: IHomeViewData | undefined = $state({});
</script>

{#if data}
    <Home
        {ls}
        basis={{
            data,
            lc_handle_farms: async () => {
                try {
                } catch (e) {
                    await handle_err(e, `lc_handle_farms`);
                }
            },
            lc_handle_products: async () => {
                try {
                } catch (e) {
                    await handle_err(e, `lc_handle_products`);
                }
            },
        }}
    />
{/if}
