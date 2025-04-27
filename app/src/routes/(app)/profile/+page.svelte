<script lang="ts">
    import { ls } from "$lib/locale/i18n";
    import { db, fs, gui, keys, radroots, route } from "$lib/util";
    import {
        handle_err,
        ndk_user,
        nostr_sync,
        Profile,
        type IViewProfileData,
    } from "@radroots/lib-app";
    import { parse_file_path, throw_err, type FilePath } from "@radroots/util";
    import { onMount } from "svelte";

    let data: IViewProfileData | undefined = $state(undefined);
    let loading_photo_upload = $state(false);
    let loading_photo_upload_open = $state(false);
    let photo_path = $state(``);

    onMount(async () => {
        try {
            const tb_nostr_profile = await db.nostr_profile_read({
                public_key: $ndk_user?.pubkey,
            });
            console.log(
                JSON.stringify(tb_nostr_profile, null, 4),
                `tb_nostr_profile`,
            );
            if (`result` in tb_nostr_profile) {
                data = { ...tb_nostr_profile.result };
                await nostr_sync.metadata({
                    metadata: tb_nostr_profile.result,
                }); // leave off await
                return;
            }
            return void (await route(`/`));
        } catch (e) {
            handle_err(e, `on_mount`);
        }
    });
</script>

<Profile
    bind:photo_path
    {ls}
    basis={{
        data,
        loading_photo_upload,
        loading_photo_upload_open,
        lc_on_destroy: async () => {
            try {
                const tb_nostrprofile = await db.nostr_profile_read({
                    public_key: $ndk_user?.pubkey,
                });
                if (`err` in tb_nostrprofile) throw_err(tb_nostrprofile); //@todo
                await nostr_sync.metadata({
                    metadata: tb_nostrprofile.result,
                }); // leave off await
            } catch (e) {
                await handle_err(e, `lc_on_destroy`);
            }
        },
        lc_handle_back: async ({ is_photo_existing }) => {
            try {
                const public_key = $ndk_user?.pubkey;
                if (!photo_path || !public_key) return void (await route(`/`));
                const keys_nostr_read = await keys.nostr_read(public_key);
                if (`err` in keys_nostr_read) throw_err(keys_nostr_read);
                if (photo_path) {
                    const confirm = await gui.confirm({
                        message: is_photo_existing
                            ? `${$ls(`notification.profile.handle_back_with_selected_photo`)}`
                            : `${$ls(`notification.profile.handle_back_with_selected_photo_no_existing`)}`,
                        ok: `${$ls(`common.upload_photo`)}`,
                        cancel: is_photo_existing
                            ? `${$ls(`common.keep_previous`)}`
                            : `${$ls(`common.continue`)}`,
                    });
                    if (!confirm) return void (await route(`/`));
                }
                loading_photo_upload = true;
                let upload_file_path: FilePath | undefined = undefined;
                parse_file_path(photo_path);
                const tb_media_upload_existing = await db.media_image_read({
                    file_path: photo_path,
                });
                if (`result` in tb_media_upload_existing)
                    upload_file_path = parse_file_path(
                        tb_media_upload_existing.result.file_path,
                    );
                else upload_file_path = parse_file_path(photo_path);
                if (!upload_file_path) throw_err(`error.util.parse_file_path`);
                const file_data = await fs.read_bin(upload_file_path.file_path);
                const res_fetch_media_image_upload =
                    await radroots.fetch_media_image_upload({
                        file_path: upload_file_path,
                        file_data,
                        secret_key: keys_nostr_read.secret_key,
                    });
                if (`err` in res_fetch_media_image_upload)
                    throw_err(res_fetch_media_image_upload);
                const { res_base: upload_res_base, res_path: upload_res_path } =
                    res_fetch_media_image_upload;
                const tb_media_image_create = await db.media_image_create({
                    file_path: upload_file_path.file_path,
                    res_base: upload_res_base,
                    res_path: upload_res_path,
                    mime_type: upload_file_path.mime_type,
                });
                if (`err` in tb_media_image_create)
                    throw_err(tb_media_image_create);
                const tb_nostr_profile_update = await db.nostr_profile_update({
                    filter: { public_key },
                    fields: {
                        picture: `${upload_res_base}/${upload_res_path}.${upload_file_path.mime_type}`,
                    },
                });
                if (`err` in tb_nostr_profile_update)
                    throw_err(tb_nostr_profile_update);
                await route(`/`);
            } catch (e) {
                await handle_err(e, `lc_handle_back`);
            } finally {
                loading_photo_upload = false;
            }
        },
        lc_handle_edit_profile_field: async ({ field }) => {
            try {
                if (field === `name`) {
                    const confirm = await gui.confirm({
                        message: `${$ls(`notification.profile.update_name_confirmation`)}. ${$ls(`common.do_you_want_to_continue_q`)}`,
                        ok: `${$ls(`common.continue`)}`,
                        cancel: `${$ls(`common.cancel`)}`,
                    });
                    if (!confirm) return;
                }
                await route(`/profile/edit`, [
                    [`key_nostr`, $ndk_user?.pubkey],
                    [`field`, field],
                ]);
            } catch (e) {
                await handle_err(e, `lc_handle_edit_profile_field`);
            }
        },
        lc_handle_photo_add: async () => {
            try {
                loading_photo_upload_open = true;
                const photo_paths_open = await gui.open_photos();
                if (!photo_paths_open) return;
                const photo_path_add = photo_paths_open.results[0];
                if (photo_path_add) return photo_path_add;
            } catch (e) {
                await handle_err(e, `lc_handle_photo_add`);
            } finally {
                loading_photo_upload_open = false;
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
