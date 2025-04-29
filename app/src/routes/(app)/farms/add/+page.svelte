<script>
    import { db, route } from "$lib/util";
    import { model_location_gcs_create } from "$lib/util/model/location-gcs";
    import { FarmsAdd, handle_err } from "@radroots/lib-app";
    import { throw_err } from "@radroots/util";
</script>

<FarmsAdd
    basis={{
        callback_route: { route: `/farms` },
        on_submit: async ({ data_s }) => {
            try {
                console.log(JSON.stringify(data_s, null, 4), `data_s`);

                const farm_create = await db.farm_create({
                    name: data_s.farm_name,
                    area: data_s.farm_area
                        ? data_s.farm_area.toString()
                        : undefined,
                    area_unit: data_s.farm_area_unit,
                });
                if (`err` in farm_create) throw_err(farm_create);
                if (`err_s` in farm_create) throw_err(farm_create.err_s[0]); //@todo

                const location_gcs_create = await model_location_gcs_create({
                    geoc_r: data_s.geocode_result,
                    geol_p: data_s.geolocation_point,
                });
                if (`err` in location_gcs_create)
                    throw_err(location_gcs_create);
                if (`err_s` in location_gcs_create)
                    throw_err(location_gcs_create.err_s[0]); //@todo

                const farm_location_set = await db.farm_location_set({
                    farm: {
                        id: farm_create.id,
                    },
                    location_gcs: {
                        id: location_gcs_create.id,
                    },
                });
                if (`err` in farm_location_set) throw_err(farm_location_set);
                await route(`/farms`);
            } catch (e) {
                await handle_err(e, `on_submit`);
            }
        },
    }}
/>
