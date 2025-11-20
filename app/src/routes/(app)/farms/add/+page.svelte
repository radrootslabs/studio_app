<script>
    import { db, route } from "$lib/utils/app";
    import { geolocation_fields_from_point_with_geocode } from "$lib/utils/geo/lib";
    import { FarmsAdd } from "@radroots/apps-lib-pwa";
    import { handle_err, throw_err } from "@radroots/utils";
</script>

<FarmsAdd
    basis={{
        callback_route: { route: `/farms` },
        on_submit: async ({ payload }) => {
            try {
                console.log(JSON.stringify(payload, null, 4), `payload`);

                const farm_create = await db.farm_create({
                    name: payload.farm_name,
                    area: payload.farm_area
                        ? payload.farm_area.toString()
                        : undefined,
                    area_unit: payload.farm_area_unit,
                });
                console.log(
                    JSON.stringify(farm_create, null, 4),
                    `farm_create`,
                );
                if ("err" in farm_create) throw_err(farm_create);
                const location_fields =
                    await geolocation_fields_from_point_with_geocode({
                        geoc_r: payload.geocode_result,
                        geol_p: payload.geolocation_point,
                    });
                if ("err" in location_fields) throw_err(location_fields);
                console.log(
                    JSON.stringify(location_fields, null, 4),
                    `location_fields`,
                );
                const location_gcs_create =
                    await db.location_gcs_create(location_fields);
                if ("err" in location_gcs_create)
                    throw_err(location_gcs_create);
                const farm_location_set = await db.farm_location_set({
                    farm: {
                        id: farm_create.result.id,
                    },
                    location_gcs: {
                        id: location_gcs_create.result.id,
                    },
                });
                console.log(
                    JSON.stringify(farm_location_set, null, 4),
                    `farm_location_set`,
                );
                if (`err` in farm_location_set) throw_err(farm_location_set);
                await route(`/farms`);
            } catch (e) {
                handle_err(e, `on_submit`);
            }
        },
    }}
/>
