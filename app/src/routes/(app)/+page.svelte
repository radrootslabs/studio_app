<script lang="ts">
    import { gui, route } from "$lib/util";
    import { handle_err, Home } from "@radroots/lib-app";
    import type { ResolveAccountInfo } from "@radroots/util";

    let data: ResolveAccountInfo | undefined = $state(undefined);
</script>

<Home
    basis={{
        data: data,
        lc_handle_farms: async () => {
            try {
                await route(`/`);
            } catch (e) {
                await handle_err(e, `lc_handle_farms`);
            }
        },
        lc_handle_products: async () => {
            try {
                await gui.alert(`@todo`);
            } catch (e) {
                await handle_err(e, `lc_handle_products`);
            }
        },
    }}
/>
