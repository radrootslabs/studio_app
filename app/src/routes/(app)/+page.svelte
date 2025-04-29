<script lang="ts">
    import { route } from "$lib/util";
    import { handle_err, Home, type IViewHomeData } from "@radroots/lib-app";

    let data: IViewHomeData | undefined = $state({});
</script>

<Home
    basis={{
        data,
        on_handle_farms: async () => {
            try {
                await route(`/farms`);
            } catch (e) {
                await handle_err(e, `on_handle_farms`);
            }
        },
        on_handle_products: async () => {
            try {
            } catch (e) {
                await handle_err(e, `on_handle_products`);
            }
        },
    }}
/>
