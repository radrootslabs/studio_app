<script lang="ts">
    import { ls } from "$lib/locale/i18n";
    import { db, gui, route } from "$lib/util";
    import {
        handle_err,
        type IViewProfileEditData,
        nostr_sync,
        parse_view_profile_field_key,
        ProfileEdit,
        qp_field,
        qp_keynostr,
    } from "@radroots/lib-app";
    import { throw_err } from "@radroots/util";
    import { onMount } from "svelte";

    type LoadData = IViewProfileEditData | undefined;
    let data: LoadData = $state(undefined);

    let val_field_init = $state(``);
    let val_field = $state(``);

    onMount(async () => {
        try {
            data = await load();
            if (!data) return void (await route(`/`)); //@todo
        } catch (e) {
            handle_err(e, `on_mount`);
        }
    });

    const load = async (): Promise<LoadData> => {
        try {
            const field = parse_view_profile_field_key($qp_field);
            if (!field || !$qp_keynostr) return void (await route(`/`));
            const tb_nostr_profile = await db.nostr_profile_read({
                public_key: $qp_keynostr,
            });
            console.log(
                JSON.stringify(tb_nostr_profile, null, 4),
                `tb_nostr_profile`,
            );
            if (`err` in tb_nostr_profile) return void (await route(`/`));
            if (field in tb_nostr_profile.result)
                val_field = tb_nostr_profile.result[field] || ``;
            val_field_init = val_field;
            return {
                public_key: tb_nostr_profile.result.public_key,
                field,
            } satisfies LoadData;
        } catch (e) {
            await handle_err(e, `load`);
        }
    };
</script>

<ProfileEdit
    bind:val_field
    {ls}
    basis={{
        data,
        lc_handle_back: async ({ field, public_key }) => {
            try {
                if (val_field_init === val_field)
                    return void (await route(`/profile`));
                const confirm = await gui.confirm({
                    message: `${$ls(`notify.profile.name_update`)}. ${$ls(`common.do_you_want_to_continue_q`)}`,
                });
                if (!confirm) return;
                const nostr_profile_update = await db.nostr_profile_update({
                    filter: {
                        public_key,
                    },
                    fields: {
                        [field]: val_field,
                    },
                });
                if (`err` in nostr_profile_update)
                    throw_err(nostr_profile_update);
                const tb_nostr_profile = await db.nostr_profile_read({
                    public_key,
                });
                console.log(
                    JSON.stringify(tb_nostr_profile, null, 4),
                    `tb_nostr_profile`,
                );
                if (`err` in tb_nostr_profile) throw_err(tb_nostr_profile);
                nostr_sync.metadata({
                    metadata: tb_nostr_profile.result,
                }); // leave off await
                await route(`/profile`);
            } catch (e) {
                await handle_err(e, `lc_handle_back`);
            }
        },
        lc_handle_input: async () => {
            try {
            } catch (e) {
                await handle_err(e, `lc_handle_input`);
            }
        },
    }}
/>
