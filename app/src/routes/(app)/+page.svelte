<script lang="ts">
    import { ls } from "$lib/locale/i18n";
    import { route } from "$lib/util";
    import { handle_err, Home, type IViewHomeData } from "@radroots/lib-app";

    let data: IViewHomeData | undefined = $state({});
</script>

<Home
    {ls}
    basis={{
        data,
        lc_handle_farms: async () => {
            try {
                await route(`/farms`);
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
