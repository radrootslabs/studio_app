<script lang="ts">
    import { db, fs } from "$lib/client";
    import ImageUploadAddPhoto from "$lib/components/image_upload_add_photo.svelte";
    import { ascii } from "$lib/conf";
    import { kv_init_page } from "$lib/util/kv";
    import type { NostrProfile } from "@radroots/models";
    import {
        app_nostr_key,
        Glyph,
        ImageBlob,
        LayoutView,
        Nav,
        t,
    } from "@radroots/svelte-lib";
    import { onMount } from "svelte";

    type LoadData = {
        nostr_profile: NostrProfile;
    };
    let ld: LoadData | undefined = undefined;

    let opt_photo_path = ``;
    let opt_display: `photos` | `following` | `followers` = `photos`;

    onMount(async () => {
        try {
            await init_page();
        } catch (e) {
        } finally {
        }
    });

    $: photo_overlay_visible = ld?.nostr_profile?.picture || opt_photo_path;
    $: classes_photo_overlay_wrap = photo_overlay_visible
        ? `bg-white/30 backdrop-blur-sm`
        : ``;
    $: classes_photo_overlay_glyph = photo_overlay_visible
        ? `text-white`
        : `text-layer-0-glyph`;

    $: classes_photo_overlay_glyph_d = photo_overlay_visible
        ? `text-white/40`
        : `text-layer-0-glyph`;
    const init_page = async (): Promise<void> => {
        try {
            await kv_init_page();
            ld = await load_data();
        } catch (e) {
            console.log(`(error) init_page `, e);
        }
    };

    const load_data = async (): Promise<LoadData | undefined> => {
        try {
            let nostr_profile_get_one = await db.nostr_profile_get_one({
                public_key: $app_nostr_key,
            });
            if (`err` in nostr_profile_get_one) return;

            const load: LoadData = {
                nostr_profile: nostr_profile_get_one.result,
            };
            return load;
        } catch (e) {
            console.log(`(error) load_data `, e);
        }
    };
</script>

<LayoutView>
    <div
        class={`relative flex flex-col min-h-[525px] h-[525px] w-full justify-center items-center bg-layer-2-surface`}
    >
        {#if opt_photo_path}
            {#await fs.read_bin(opt_photo_path) then file_data}
                <ImageBlob
                    basis={{
                        data: file_data,
                    }}
                />
            {/await}
        {:else}
            <div
                class={`flex flex-row justify-start items-center -translate-y-8`}
            >
                <ImageUploadAddPhoto bind:photo_path={opt_photo_path} />
            </div>
        {/if}
        <div
            class={`absolute bottom-0 left-0 flex flex-col h-[calc(100%-100%/1.618)] w-full px-6 gap-2 justify-end items-center ${classes_photo_overlay_wrap}`}
        >
            <div
                class={`flex flex-col w-full gap-[2px] justify-center items-center`}
            >
                <div
                    class={`flex flex-row h-10 w-full justify-start items-center`}
                >
                    <button
                        class={`group flex flex-row justify-center items-center`}
                        on:click={async () => {}}
                    >
                        <p
                            class={`font-sansd font-[600] text-[1.7rem] ${classes_photo_overlay_glyph} ${ld?.nostr_profile.display_name ? `` : `capitalize opacity-active`} el-re`}
                        >
                            {ld?.nostr_profile.display_name
                                ? ld.nostr_profile.display_name
                                : `+ ${`${$t(`icu.add_*`, { value: `${$t(`common.profile_name`)}` })}`}`}
                        </p>
                    </button>
                </div>
                <div
                    class={`flex flex-row w-full gap-[6px] justify-start items-center`}
                >
                    <button
                        class={`group flex flex-row justify-center items-center`}
                        on:click={async () => {}}
                    >
                        <p
                            class={`font-sans font-[600] text-[1.1rem] ${classes_photo_overlay_glyph} ${ld?.nostr_profile.name ? `` : `capitalize opacity-active`} el-re`}
                        >
                            {ld?.nostr_profile.name
                                ? `@${ld.nostr_profile.name}`
                                : `+ ${`${$t(`icu.add_*`, { value: `${$t(`common.username`)}` })}`}`}
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
                        on:click={async () => {}}
                    >
                        <p
                            class={`font-sans font-[400] text-[1.1rem] ${classes_photo_overlay_glyph} ${ld?.nostr_profile.about ? `` : `capitalize opacity-active`}`}
                        >
                            {ld?.nostr_profile.about
                                ? `@${ld.nostr_profile.about}`
                                : `+ ${`${$t(`icu.add_*`, { value: `${$t(`common.bio`)}` })}`}`}
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
                        opt_display = `photos`;
                    }}
                >
                    <p
                        class={`font-sans text-[1.1rem] font-[600] ${opt_display === `photos` ? `text-layer-1-glyph_d` : `text-layer-0-glyph`} el-re`}
                    >
                        {`Photos`}
                    </p>
                </button>
                <button
                    class={`flex flex-row justify-center items-center`}
                    on:click={async () => {
                        opt_display = `following`;
                    }}
                >
                    <p
                        class={`font-sans text-[1.1rem] font-[600] ${opt_display === `following` ? `text-layer-1-glyph_d` : `text-layer-0-glyph`} el-re`}
                    >
                        {`Following`}
                    </p>
                </button>
                <button
                    class={`flex flex-row justify-center items-center`}
                    on:click={async () => {
                        opt_display = `followers`;
                    }}
                >
                    <p
                        class={`font-sans text-[1.1rem] font-[600] ${opt_display === `followers` ? `text-layer-1-glyph_d` : `text-layer-0-glyph`} el-re`}
                    >
                        {`Followers`}
                    </p>
                </button>
            </div>
        </div>
    </div>
    <div class={`flex flex-col w-full justify-start items-center`}>
        {#if opt_display === `photos`}
            <p class={`font-sans font-[400] text-layer-0-glyph`}>
                {`photos `.repeat(500)}
            </p>
        {:else if opt_display === `following`}
            <p class={`font-sans font-[400] text-layer-0-glyph`}>
                {`following `.repeat(500)}
            </p>
        {:else if opt_display === `followers`}
            <p class={`font-sans font-[400] text-layer-0-glyph`}>
                {`followers `.repeat(500)}
            </p>
        {/if}
    </div>
</LayoutView>
<Nav
    basis={{
        prev: {
            label: `${$t(`common.home`)}`,
            route: `/`,
        },
        title: {
            label: {
                value: `${$t(`common.profile`)}`,
            },
        },
    }}
/>
