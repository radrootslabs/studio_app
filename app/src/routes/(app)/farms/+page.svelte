<script lang="ts">
    import { locale, ls } from "$lib/locale/i18n";
    import { route } from "$lib/util";
    import { lc_geocode } from "$lib/util/callback";
    import { Farms, handle_err, type IViewFarmsData } from "@radroots/lib-app";

    let data: IViewFarmsData | undefined = $state(undefined);
</script>

<Farms
    {ls}
    {locale}
    basis={{
        data,
        callback_route: { route: `/` },
        lc_geocode,
        lc_handle_farm_add: async () => {
            try {
                await route(`/farms/add`);
            } catch (e) {
                await handle_err(e, `lc_handle_farm_add`);
            }
        },
        lc_handle_farm_view: async (farm_id) => {
            try {
            } catch (e) {
                await handle_err(e, `lc_handle_farm_view`);
            }
        },
    }}
/>
