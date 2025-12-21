<script lang="ts">
    import { db, fs, nostr_keys, notif, radroots, route } from "$lib/utils/app";
    import { ls } from "$lib/utils/i18n";
    import { ndk_user, parse_file_path } from "@radroots/apps-lib";
    import { Profile } from "@radroots/apps-lib-pwa";
    import type { IViewProfileData } from "@radroots/apps-lib-pwa/types/views/profile";
    import { handle_err, throw_err } from "@radroots/utils";
    import { onMount } from "svelte";

    let data: IViewProfileData | undefined = $state(undefined);
    let loading_photo_upload = $state(false);
    let loading_photo_upload_open = $state(false);
    let photo_path = $state(``);

    onMount(async () => {
        try {
            //    await init();
            await db.init();
            data = await load_data();
        } catch (e) {
            handle_err(e, `on_mount`);
            //   await route(`/`);
        }
    });

    const init = async (): Promise<void> => {
        await db.init();
    };

    const load_data = async (): Promise<IViewProfileData | undefined> => {
        const nostr_profile = await db.nostr_profile_find_one({
            on: {
                public_key: $ndk_user?.pubkey,
            },
        });
        if ("err" in nostr_profile) throw_err(nostr_profile);
        //await nostr_sync.metadata({ metadata: tb_nostr_profile.result }); // no await
        return { profile: nostr_profile.result };
    };
</script>

<Profile
    bind:photo_path
    basis={{
        data,
        loading_photo_upload,
        loading_photo_upload_open,
        on_destroy: async () => {
            try {
                const tb_nostrprofile = await db.nostr_profile_find_one({
                    on: {
                        public_key: $ndk_user?.pubkey,
                    },
                });
                if ("err" in tb_nostrprofile) throw_err(tb_nostrprofile); // @todo
                /*await nostr_sync.metadata({
                    metadata: tb_nostrprofile.result,
                }); // no await */
            } catch (e) {
                handle_err(e, `on_destroy`);
            }
        },
        on_handle_back: async ({ is_photo_existing }) => {
            try {
                if (!photo_path || !$ndk_user?.pubkey)
                    return void (await route(`/`));
                const nostr_key = await nostr_keys.read($ndk_user.pubkey);
                if ("err" in nostr_key) throw_err(nostr_key);
                /*if (photo_path) {
                    const confirm = await notif.confirm({
                        message: is_photo_existing
                            ? `${$ls(
                                  `notification.profile.handle_back_with_selected_photo`,
                              )}`
                            : `${$ls(
                                  `notification.profile.handle_back_with_selected_photo_no_existing`,
                              )}`,
                        ok: `${$ls(`common.upload_photo`)}`,
                        cancel: is_photo_existing
                            ? `${$ls(`common.keep_previous`)}`
                            : `${$ls(`common.continue`)}`,
                    });
                    if (!confirm) return void (await route(`/`));
                }*/

                loading_photo_upload = true;
                const parsed_photo_path = parse_file_path(photo_path);
                if (!parsed_photo_path) throw_err("error.upload.no_photo_path");

                const upload_path =
                    "blob_path" in parsed_photo_path
                        ? parsed_photo_path.blob_path
                        : parsed_photo_path.file_path;
                const upload_name =
                    "blob_path" in parsed_photo_path
                        ? parsed_photo_path.blob_name
                        : parsed_photo_path.file_name;

                const profile_photo_curr = await db.media_image_find_one({
                    on: {
                        file_path: upload_name,
                    },
                });
                if ("err" in profile_photo_curr) throw_err(profile_photo_curr);
                if (
                    "result" in profile_photo_curr &&
                    profile_photo_curr.result
                ) {
                    alert("todo, photo already added");
                    return;
                    //upload_file_path = parse_file_path(
                    //    profile_photo_curr.result.file_path,
                    //);
                }

                const upload_bin = await fs.read_bin(upload_path);
                if ("err" in upload_bin) throw_err(upload_bin);

                const media_upload = await radroots.media_image_upload({
                    file_data: upload_bin,
                    secret_key: nostr_key.secret_key,
                });
                console.log(
                    JSON.stringify(media_upload, null, 4),
                    `media_upload`,
                );
                if ("err" in media_upload) throw_err(media_upload);

                const media_image = await db.media_image_create({
                    file_path: upload_path,
                    res_base: media_upload.base_url,
                    res_path: media_upload.hash,
                    mime_type: media_upload.ext,
                });
                console.log(
                    JSON.stringify(media_image, null, 4),
                    `media_image`,
                );

                if ("err" in media_image) throw_err(media_image);

                const tb_nostr_profile_update = await db.nostr_profile_update({
                    on: { public_key: $ndk_user.pubkey },
                    fields: {
                        picture: `${media_upload.base_url}/${media_upload.hash}.${media_upload.ext}`,
                    },
                });
                console.log(
                    JSON.stringify(tb_nostr_profile_update, null, 4),
                    `tb_nostr_profile_update`,
                );
                if ("err" in tb_nostr_profile_update)
                    throw_err(tb_nostr_profile_update);
                await route(`/`);

                //let upload_bin: WebFilePath | undefined = undefined;
                //if ("blob_path" in upload_path)
                //
                // else upload_bin = await fs.read_bin(upload_path.file_path);

                /*

                console.log(`photo_path `, photo_path);

                console.log(`photo_path_bin `, photo_path_bin);

                let file_path_upload: FilePath | undefined = undefined;
                console.log(`upload_file_path `, upload_file_path);

              

                console.log(
                    JSON.stringify(profile_photo_curr, null, 4),
                    `profile_photo_curr`,
                );

            
                else upload_file_path = parse_file_path(photo_path);
                if (!upload_file_path) throw_err(`error.util.parse_file_path`);
                const file_data = await fs.read_bin(upload_file_path.file_path);
                if ("err" in file_data) throw_err(file_data);
              
                */
            } catch (e) {
                handle_err(e, `on_handle_back`);
            } finally {
                loading_photo_upload = false;
            }
        },
        on_handle_edit_profile_field: async ({ field }) => {
            try {
                if (field === `name`) {
                    const confirm = await notif.confirm({
                        message: `${$ls(
                            `notification.profile.update_name_confirmation`,
                        )}. ${$ls(`common.do_you_want_to_continue_q`)}`,
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
                handle_err(e, `on_handle_edit_profile_field`);
            }
        },
        on_handle_photo_options: async () => {
            try {
            } catch (e) {
                handle_err(e, `on_handle_photo_options`);
            }
        },
    }}
/>
