<script lang="ts">
    import { db, notif, route } from "$lib/utils/app";
    import { ls } from "$lib/utils/i18n";
    import { ProfileEdit } from "@radroots/apps-lib-pwa";
    import { qp_field, qp_keynostr } from "@radroots/apps-lib-pwa/stores/app";
    import type { IViewProfileEditData } from "@radroots/apps-lib-pwa/types/views/profile";
    import { parse_view_profile_field_key } from "@radroots/apps-lib-pwa/utils/profile/lib";
    import { handle_err, throw_err } from "@radroots/utils";
    import { onMount } from "svelte";

    type LoadData = IViewProfileEditData | undefined;
    let data: LoadData = $state(undefined);

    let val_field_init = $state(``);
    let val_field = $state(``);

    onMount(async () => {
        try {
            data = await load_data();
        } catch (e) {
            handle_err(e, `on_mount`);
            await route(`/`);
        }
    });

    const load_data = async (): Promise<LoadData | undefined> => {
        const field = parse_view_profile_field_key($qp_field);
        if (!field || !$qp_keynostr) throw_err("*-params");
        const nostr_profile = await db.nostr_profile_find_one({
            on: {
                public_key: $qp_keynostr,
            },
        });
        if ("err" in nostr_profile) throw_err(nostr_profile);
        if (field in nostr_profile.result)
            val_field = nostr_profile.result[field] || ``;
        val_field_init = val_field;
        const data: LoadData = {
            public_key: nostr_profile.result.public_key,
            field,
        };
        return data;
    };
</script>

<ProfileEdit
    bind:val_field
    basis={{
        data,
        on_handle_back: async ({ field, public_key }) => {
            try {
                if (val_field_init === val_field)
                    return void (await route(`/profile`));
                const confirm = await notif.confirm({
                    message: `${$ls(
                        `notification.profile.update_name_confirmation`,
                    )}. ${$ls(`common.do_you_want_to_continue_q`)}`,
                });
                if (!confirm) return;
                const nostr_profile_update = await db.nostr_profile_update({
                    on: {
                        public_key,
                    },
                    fields: {
                        [field]: val_field,
                    },
                });
                if ("err" in nostr_profile_update)
                    throw_err(nostr_profile_update);
                const tb_nostr_profile = await db.nostr_profile_find_one({
                    on: { public_key },
                });
                console.log(
                    JSON.stringify(tb_nostr_profile, null, 4),
                    `tb_nostr_profile`,
                );
                if ("err" in tb_nostr_profile) throw_err(tb_nostr_profile);
                //nostr_sync.metadata({ metadata: tb_nostr_profile.result }); // no await
                await route(`/profile`);
            } catch (e) {
                handle_err(e, `on_handle_back`);
            }
        },
        on_handle_input: async () => {
            try {
            } catch (e) {
                handle_err(e, `on_handle_input`);
            }
        },
    }}
/>
