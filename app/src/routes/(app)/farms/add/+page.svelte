<script>
    import { locale, ls } from "$lib/locale/i18n";
    import {
        lc_geocode,
        lc_geop_current,
        lc_gui_alert,
    } from "$lib/util/callback";
    import { FarmsAdd, handle_err } from "@radroots/lib-app";
</script>

<FarmsAdd
    {ls}
    {locale}
    basis={{
        callback_route: { route: `/farms` },
        lc_gui_alert,
        lc_geocode,
        lc_geop_current,
        lc_submit: async ({ data_s }) => {
            try {
                console.log(JSON.stringify(data_s, null, 4), `data_s`);
            } catch (e) {
                await handle_err(e, `lc_submit`);
            }
        },
    }}
/>
