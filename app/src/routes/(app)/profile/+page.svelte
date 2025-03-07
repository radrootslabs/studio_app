<script lang="ts">
    import { ls } from "$lib/locale/i18n";
    import { db, fs, gui, nostrsync, route } from "$lib/util";
    import { handle_err, ndk_user, Profile } from "@radroots/lib-app";
    import { throw_err } from "@radroots/util";

    let loading_photo_upload = $state(false);
    let photo_path_opt = $state(``);
</script>

<Profile
    bind:photo_path_opt
    {ls}
    basis={{
        loading_photo_upload,
        lc_on_destroy: async () => {
            try {
                const tb_nostrprofile = await db.nostr_profile_read({
                    public_key: $ndk_user.pubkey,
                });
                if (`err` in tb_nostrprofile) throw_err(tb_nostrprofile.err); //@todo
                const ev_metadata = await nostrsync.metadata({
                    metadata: tb_nostrprofile.result,
                });
                if (`err` in ev_metadata)
                    throw_err(`error.nostr.sync.missing_metadata_event`);
                await ev_metadata.publish();
            } catch (e) {
                await handle_err(e, `lc_on_destroy`);
            }
        },
        lc_handle_back: async () => {
            try {
                if (photo_path_opt) {
                    const confirm = await gui.confirm({
                        message: `${$ls(`notification.profile.update_name_confirmation_on_route_previous`)}`,
                        ok: `${$ls(`common.upload_photo`)}`,
                        cancel: `${$ls(`common.keep_previous`)}`,
                    });
                    if (!confirm) return void (await route(`/`));
                    // @todo upload photo
                }
                await route(`/`);
            } catch (e) {
                await handle_err(e, `lc_handle_back`);
            }
        },
        lc_handle_edit_profile_field: async ({ field }) => {
            try {
                //@todo
                if (field === `name`) {
                    const confirm = await gui.confirm({
                        message: `${$ls(`notification.profile.update_name_confirmation`)}. ${$ls(`common.do_you_want_to_continue_q`)}`,
                        ok: `${$ls(`common.continue`)}`,
                        cancel: `${$ls(`common.cancel`)}`,
                    });
                    if (!confirm) return;
                }
                await route(`/profile/edit`, [
                    [`key_nostr`, $ndk_user.pubkey],
                    [`field`, field],
                ]);
            } catch (e) {
                await handle_err(e, `lc_handle_edit_profile_field`);
            }
        },
        lc_handle_photo_add: async () => {
            try {
                loading_photo_upload = true;
                const photo_paths_open = await gui.open_photos();
                if (!photo_paths_open) return;
                const photo_path_add = photo_paths_open.results[0];
                if (photo_path_add) return photo_path_add;
            } catch (e) {
                await handle_err(e, `lc_handle_photo_add`);
            } finally {
                loading_photo_upload = false;
            }
        },
        lc_handle_photo_options: async () => {
            try {
            } catch (e) {
                await handle_err(e, `lc_handle_photo_options`);
            }
        },
        lc_fs_read_bin: async (file_path) => {
            try {
                return await fs.read_bin(file_path);
            } catch (e) {
                await handle_err(e, `lc_fs_read_bin`);
            }
        },
    }}
/>
