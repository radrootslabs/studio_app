<script lang="ts">
    import { db, dialog, fs } from "$lib/client";
    import ImageUploadAddPhoto from "$lib/components/image_upload_add_photo.svelte";
    import { throw_err } from "$lib/util/client";
    import { kv_init_page } from "$lib/util/kv";
    import { model_media_upload_add_list } from "$lib/util/models-media-upload";
    import { nostr_sync_metadata } from "$lib/util/nostr-sync";
    import { fmt_media_upload_url, type NostrProfile } from "@radroots/models";
    import {
        app_nostr_key,
        ascii,
        catch_err,
        Glyph,
        ImageBlob,
        ImagePath,
        Loading,
        ls,
        route,
        TabsFloat,
    } from "@radroots/svelte-lib";
    import { onDestroy, onMount } from "svelte";

    type LoadData = {
        nostr_profile: NostrProfile;
    };
    let ld: LoadData | undefined = undefined;

    let loading_photo_upload = false;
    let opt_photo_path = ``;
    type ViewDisplay = `photos` | `following` | `followers`;
    let view_display: ViewDisplay = `photos`;

    $: {
        console.log(JSON.stringify(ld, null, 4), `ld`);
    }

    onMount(async () => {
        try {
            await init_page();
        } catch (e) {
            await catch_err(e, `mount`);
        } finally {
        }
    });

    onDestroy(async () => {
        try {
            await nostr_sync_metadata();
        } catch (e) {
            await catch_err(e, `destroy`);
        } finally {
        }
    });

    $: photo_overlay_visible = ld?.nostr_profile?.picture || opt_photo_path;
    $: classes_photo_overlay_glyph = photo_overlay_visible
        ? `text-white`
        : `text-layer-0-glyph`;
    $: classes_photo_overlay_glyph_opt = photo_overlay_visible
        ? `text-gray-300`
        : `text-layer-0-glyph`;
    $: classes_photo_overlay_glyph_opt_selected = photo_overlay_visible
        ? `text-white`
        : `text-layer-1-glyph`;

    const init_page = async (): Promise<void> => {
        try {
            await kv_init_page();
            ld = await load_data();
        } catch (e) {
            await catch_err(e, `init_page`);
        }
    };

    const load_data = async (): Promise<LoadData | undefined> => {
        try {
            let nostr_profile_get_one = await db.nostr_profile_get_one({
                public_key: $app_nostr_key,
            });
            if (`err` in nostr_profile_get_one)
                return throw_err(nostr_profile_get_one);
            return {
                nostr_profile: nostr_profile_get_one.result,
            } satisfies LoadData;
        } catch (e) {
            await catch_err(e, `load_data`);
        }
    };

    const handle_back = async (): Promise<void> => {
        try {
            if (opt_photo_path) await handle_profile_photo_add(opt_photo_path);
            await route(`/`);
        } catch (e) {
            await catch_err(e, `handle_back`);
        }
    };

    const handle_profile_photo_add = async (
        file_path: string,
    ): Promise<void> => {
        try {
            const confirm = await dialog.confirm({
                message: `The photo will be used for your profile. Do you want to continue?`,
            });
            if (!confirm) return;
            loading_photo_upload = true;
            let photo_url = ``;
            const media_upload_existing = await db.media_upload_get_one({
                file_path,
            });
            if (`result` in media_upload_existing)
                photo_url = fmt_media_upload_url(media_upload_existing.result);
            else {
                const media_uploads = await model_media_upload_add_list({
                    photo_paths: [file_path],
                });
                if (`alert` in media_uploads) {
                    await dialog.alert(media_uploads.alert);
                    return;
                } else if (`confirm` in media_uploads) {
                    await dialog.confirm(media_uploads.confirm);
                    return;
                }
                if (
                    `results` in media_uploads &&
                    media_uploads.results.length
                ) {
                    const media_upload = await db.media_upload_get_one({
                        id: media_uploads.results[0],
                    });
                    if (`result` in media_upload)
                        photo_url = fmt_media_upload_url(media_upload.result);
                }
            }
            if (photo_url) {
                const nostr_profile_update = await db.nostr_profile_update({
                    on: {
                        public_key: $app_nostr_key,
                    },
                    fields: {
                        picture: photo_url,
                    },
                });
                if (`err` in nostr_profile_update) {
                    await dialog.alert(`${$ls(`error.client.unhandled`)}`);
                    return;
                }
            }
            location.reload();
        } catch (e) {
            await catch_err(e, `handle_profile_photo_add`);
        } finally {
            loading_photo_upload = false;
        }
    };

    const handle_edit_display_name = async (): Promise<void> => {
        try {
            await route(`/settings/profile/edit`, [
                [`nostr_pk`, $app_nostr_key],
                [`rkey`, `display_name`],
            ]);
        } catch (e) {
            await catch_err(e, `handle_edit_display_name`);
        }
    };

    const handle_edit_name = async (): Promise<void> => {
        try {
            if (ld?.nostr_profile.name) {
                const confirm = await dialog.confirm({
                    message: `Updating your username will result in public links on your profile being updated. Do you want to continue?`,
                });
                if (!confirm) return;
            }
            await route(`/settings/profile/edit`, [
                [`nostr_pk`, $app_nostr_key],
                [`rkey`, `name`],
            ]);
        } catch (e) {
            await catch_err(e, `handle_edit_name`);
        }
    };

    const handle_edit_about = async (): Promise<void> => {
        try {
            await route(`/settings/profile/edit`, [
                [`nostr_pk`, $app_nostr_key],
                [`rkey`, `about`],
            ]);
        } catch (e) {
            await catch_err(e, `handle_edit_about`);
        }
    };
</script>

<div
    class={`relative flex flex-col min-h-[525px] h-[525px] w-full justify-center items-center bg-layer-2-surface fade-in`}
>
    {#if ld?.nostr_profile?.picture}
        <ImagePath
            basis={{
                path: ld.nostr_profile.picture,
            }}
        />
        <div class={`absolute top-16 left-6 flex flex-row`}>
            <button
                class={`flex flex-row h-12 w-12 justify-center items-center bg-layer-2-surface rounded-full el-re`}
                on:click={async () => {
                    await handle_back();
                }}
            >
                {#if loading_photo_upload}
                    <Loading />
                {:else}
                    <Glyph
                        basis={{
                            classes: `text-layer-0-glyph`,
                            dim: `sm+`,
                            weight: `bold`,
                            key: `arrow-left`,
                        }}
                    />
                {/if}
            </button>
        </div>
        <div class={`absolute top-16 right-6 flex flex-row`}>
            <button
                class={`flex flex-row h-12 w-12 justify-center items-center bg-layer-2-surface rounded-full el-re`}
                on:click={async () => {
                    alert(`@todo!`);
                }}
            >
                <Glyph
                    basis={{
                        classes: `text-layer-0-glyph`,
                        dim: `md-`,
                        weight: `bold`,
                        key: `images-square`,
                    }}
                />
            </button>
        </div>
    {:else if opt_photo_path}
        {#await fs.read_bin(opt_photo_path) then file_data}
            <ImageBlob
                basis={{
                    data: file_data,
                }}
            />
        {/await}
    {:else}
        <div class={`flex flex-row justify-start items-center -translate-y-8`}>
            <ImageUploadAddPhoto bind:photo_path={opt_photo_path} />
        </div>
    {/if}
    <div
        class={`absolute bottom-0 left-0 flex flex-col h-[calc(100%-100%/1.618)] w-full px-6 gap-2 justify-end items-center`}
    >
        <div
            class={`flex flex-col w-full gap-[2px] justify-center items-center`}
        >
            <div class={`flex flex-row h-10 w-full justify-start items-center`}>
                <button
                    class={`group flex flex-row justify-center items-center`}
                    on:click={async () => {
                        await handle_edit_display_name();
                    }}
                >
                    <p
                        class={`font-sansd font-[600] text-[2rem] ${classes_photo_overlay_glyph} ${ld?.nostr_profile.display_name ? `` : `capitalize opacity-active`} el-re`}
                    >
                        {ld?.nostr_profile.display_name
                            ? ld.nostr_profile.display_name
                            : `+ ${`${$ls(`icu.add_*`, { value: `${$ls(`common.profile_name`)}` })}`}`}
                    </p>
                </button>
            </div>
            <div
                class={`flex flex-row w-full gap-[6px] justify-start items-center`}
            >
                <button
                    class={`group flex flex-row justify-center items-center`}
                    on:click={async () => {
                        await handle_edit_name();
                    }}
                >
                    <p
                        class={`font-sansd font-[600] text-[1.1rem] ${classes_photo_overlay_glyph} ${ld?.nostr_profile.name ? `` : `capitalize opacity-active`} el-re`}
                    >
                        {ld?.nostr_profile.name
                            ? `@${ld.nostr_profile.name}`
                            : `+ ${`${$ls(`icu.add_*`, { value: `${$ls(`common.username`)}` })}`}`}
                    </p>
                </button>
                <p
                    class={`font-sans font-[400] ${classes_photo_overlay_glyph}`}
                >
                    {ascii.bullet}
                </p>
                <button
                    class={`flex flex-row justify-center items-center`}
                    on:click={async () => {
                        alert(`@todo!`);
                    }}
                >
                    <Glyph
                        basis={{
                            classes: `${classes_photo_overlay_glyph}`,
                            dim: `xs`,
                            weight: `bold`,
                            key: `link-simple`,
                        }}
                    />
                </button>
            </div>
            <div class={`flex flex-row w-full justify-start items-center`}>
                <button
                    class={`group flex flex-row justify-center items-center`}
                    on:click={async () => {
                        await handle_edit_about();
                    }}
                >
                    <p
                        class={`font-sansd font-[400] text-[1.1rem] ${classes_photo_overlay_glyph} ${ld?.nostr_profile.about ? `` : `capitalize opacity-active`}`}
                    >
                        {ld?.nostr_profile.about
                            ? `@${ld.nostr_profile.about}`
                            : `+ ${`${$ls(`icu.add_*`, { value: `${$ls(`common.bio`)}` })}`}`}
                    </p>
                </button>
            </div>
        </div>
        <div
            class={`flex flex-row w-full pt-2 pb-6 gap-2 justify-start items-center`}
        >
            <button
                class={`flex flex-row justify-center items-center`}
                on:click={async () => {
                    view_display = `photos`;
                }}
            >
                <p
                    class={`font-sans text-[1.1rem] font-[600] capitalize ${view_display === `photos` ? classes_photo_overlay_glyph_opt_selected : classes_photo_overlay_glyph_opt} el-re`}
                >
                    {`photos`}
                </p>
            </button>
            <button
                class={`flex flex-row justify-center items-center`}
                on:click={async () => {
                    view_display = `following`;
                }}
            >
                <p
                    class={`font-sans text-[1.1rem] font-[600] capitalize ${view_display === `following` ? classes_photo_overlay_glyph_opt_selected : classes_photo_overlay_glyph_opt} el-re`}
                >
                    {`following`}
                </p>
            </button>
            <button
                class={`flex flex-row justify-center items-center`}
                on:click={async () => {
                    view_display = `followers`;
                }}
            >
                <p
                    class={`font-sans text-[1.1rem] font-[600] capitalize ${view_display === `followers` ? classes_photo_overlay_glyph_opt_selected : classes_photo_overlay_glyph_opt} el-re`}
                >
                    {`followers`}
                </p>
            </button>
        </div>
    </div>
</div>
<div class={`flex flex-col w-full justify-start items-center`}>
    {#if view_display === `photos`}
        <p class={`font-sans font-[400] text-layer-0-glyph`}>
            {view_display}
        </p>
    {:else if view_display === `following`}
        <p class={`font-sans font-[400] text-layer-0-glyph`}>
            {view_display}
        </p>
    {:else if view_display === `followers`}
        <p class={`font-sans font-[400] text-layer-0-glyph`}>
            {view_display}
        </p>
    {/if}
</div>
<TabsFloat />
