<script lang="ts">
    import { ls } from "$lib/locale/i18n";
    import { cfg_role } from "$lib/store";
    import { gui, route } from "$lib/util";
    import { handle_err, Home, type IHomeViewData } from "@radroots/lib-app";

    let data: IHomeViewData | undefined = $state({});
</script>

{#if data}
    {#if $cfg_role === `farmer`}
        <div class={`flex flex-col pt-20 justify-start items-center`}>
            <p class={`font-sans font-[400] text-layer-0-glyph`}>
                {`farmer`}
            </p>
        </div>
    {:else if $cfg_role === `public`}
        <Home
            {ls}
            basis={{
                data,
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
    {/if}
{/if}
