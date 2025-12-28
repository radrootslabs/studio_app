<script>
    import { db, route } from "$lib/utils/app";
    import {
        geolocation_fields_from_point,
        geolocation_fields_from_point_with_geocode
    } from "$lib/utils/geo/lib";
    import { FarmsAdd } from "@radroots/apps-lib-pwa";
    import { nostr_pubkey } from "@radroots/apps-nostr";
    import { d_tag_create, handle_err, throw_err } from "@radroots/utils";
</script>

<FarmsAdd
    basis={{
        callback_route: { route: `/farms` },
        on_submit: async ({ payload }) => {
            try {
                console.log(JSON.stringify(payload, null, 4), `payload`);

                const pubkey_val = $nostr_pubkey;
                if (!pubkey_val) throw_err(`missing_nostr_pubkey`);
                const farm_d_tag = d_tag_create();
                const farm_create = await db.farm_create({
                    d_tag: farm_d_tag,
                    pubkey: pubkey_val,
                    name: payload.farm_name,
                    about: payload.farm_about,
                    website: payload.farm_website,
                });
                console.log(
                    JSON.stringify(farm_create, null, 4),
                    `farm_create`,
                );
                if ("err" in farm_create) throw_err(farm_create);
                if (payload.geolocation_point) {
                    const location_fields = payload.geocode_result
                        ? await geolocation_fields_from_point_with_geocode({
                              geoc_r: payload.geocode_result,
                              geol_p: payload.geolocation_point,
                              label: payload.farm_location_label,
                          })
                        : await geolocation_fields_from_point({
                              geol_p: payload.geolocation_point,
                              label: payload.farm_location_label,
                          });
                    if ("err" in location_fields) throw_err(location_fields);
                    console.log(
                        JSON.stringify(location_fields, null, 4),
                        `location_fields`,
                    );
                    const gcs_location_create =
                        await db.gcs_location_create(location_fields);
                    if ("err" in gcs_location_create)
                        throw_err(gcs_location_create);
                    const farm_gcs_location_create =
                        await db.farm_gcs_location_create({
                            farm_id: farm_create.result.id,
                            gcs_location_id: gcs_location_create.result.id,
                            role: `primary`,
                        });
                    console.log(
                        JSON.stringify(farm_gcs_location_create, null, 4),
                        `farm_gcs_location_create`,
                    );
                    if ("err" in farm_gcs_location_create)
                        throw_err(farm_gcs_location_create);
                }
                await route(`/farms`);
            } catch (e) {
                handle_err(e, `on_submit`);
            }
        },
    }}
/>
