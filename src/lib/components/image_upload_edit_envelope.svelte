<script lang="ts">
    import { fs } from "$lib/client";
    import {
        catch_err,
        envelope_tilt,
        envelope_visible,
        EnvelopeLower,
        ImageBlob,
        ls,
        time_iso,
    } from "@radroots/svelte-lib";
    import {
        format_file_bytes,
        list_move_index,
        parse_file_name,
    } from "@radroots/utils";
    import { onDestroy, onMount } from "svelte";

    export let photo_paths: string[];
    export let photo_edit: { index: number; file_path: string } | undefined;

    $: envelope_visible.set(!!photo_edit);

    onMount(async () => {
        try {
            envelope_tilt.set(false);
        } catch (e) {
        } finally {
        }
    });

    onDestroy(async () => {
        try {
            envelope_tilt.set(true);
        } catch (e) {
        } finally {
        }
    });

    const handle_photo_edit_move = async (
        opts_photo_index: number,
    ): Promise<void> => {
        try {
            photo_paths = list_move_index(photo_paths, opts_photo_index, 0);
        } catch (e) {
            await catch_err(e, `handle_photo_edit_move`);
        }
    };
</script>

<EnvelopeLower
    basis={{
        close: async () => {
            photo_edit = undefined;
        },
    }}
>
    {#if photo_edit}
        <div
            class={`flex flex-col w-full px-4 gap-4 justify-start items-center`}
        >
            <div
                class={`flex flex-row w-full justify-center items-center round-44 overflow-hidden`}
            >
                {#await fs.read_bin(photo_edit.file_path) then file_data}
                    <ImageBlob
                        basis={{
                            data: file_data,
                        }}
                    />
                {/await}
            </div>
            <div
                class={`flex flex-col w-full pb-16 gap-4 justify-center items-center`}
            >
                <button
                    class={`flex flex-row h-touch_guide w-full justify-center items-center rounded-2xl bg-layer-1-glyph-hl`}
                    on:click={async () => {
                        if (photo_edit?.index === 0) return;
                        else if (photo_edit)
                            await handle_photo_edit_move(photo_edit.index);
                        photo_edit = undefined;
                    }}
                >
                    <p class={`font-sans font-[600] text-lg text-white`}>
                        {#if photo_edit.index === 0}
                            {`${$ls(`icu.primary_*`, { value: `${$ls(`common.photo`)}`.toLowerCase() })}`}
                        {:else}
                            {`${$ls(`common.make_primary`)}`}
                        {/if}
                    </p>
                </button>
                {#await fs.info(photo_edit.file_path) then fs_info}
                    {#if fs_info}
                        <div
                            class={`flex flex-col w-full px-4 gap-3 justify-start items-start`}
                        >
                            <div
                                class={`flex flex-col gap-1 justify-start items-start`}
                            >
                                <p
                                    class={`col-span-8 font-sans font-[400] text-[1.05rem] text-layer-0-glyph`}
                                >
                                    {`${$ls(`common.file_name`)}:`}
                                </p>
                                <p
                                    class={`col-span-8 font-sans font-[400] text-[1.05rem] text-layer-0-glyph`}
                                >
                                    {`${parse_file_name(photo_edit.file_path)}`}
                                </p>
                            </div>
                            <div
                                class={`flex flex-col gap-1 justify-start items-start`}
                            >
                                <p
                                    class={`col-span-8 font-sans font-[400] text-[1.05rem] text-layer-0-glyph`}
                                >
                                    {`${$ls(`common.file_size`)}:`}
                                </p>
                                <p
                                    class={`col-span-8 font-sans font-[400] text-[1.05rem] text-layer-0-glyph`}
                                >
                                    {`${format_file_bytes(fs_info.size, `mb`)}`}
                                </p>
                            </div>
                            <div
                                class={`flex flex-col gap-1 justify-start items-start`}
                            >
                                <p
                                    class={`col-span-8 font-sans font-[400] text-[1.05rem] text-layer-0-glyph`}
                                >
                                    {`${$ls(`common.date_created`)}:`}
                                </p>
                                <p
                                    class={`col-span-8 font-sans font-[400] text-[1.05rem] text-layer-0-glyph`}
                                >
                                    {`${time_iso(fs_info.birthtime?.toISOString(), `file_info`).replaceAll(`,`, ` ${`${$ls(`common.at`)}`.toLowerCase()}`)}`}
                                </p>
                            </div>
                            <div
                                class={`flex flex-col gap-1 justify-start items-start`}
                            >
                                <p
                                    class={`col-span-8 font-sans font-[400] text-[1.05rem] text-layer-0-glyph`}
                                >
                                    {`${$ls(`common.date_modified`)}:`}
                                </p>
                                <p
                                    class={`col-span-8 font-sans font-[400] text-[1.05rem] text-layer-0-glyph`}
                                >
                                    {`${time_iso(fs_info.mtime?.toISOString(), `file_info`).replaceAll(`,`, ` ${`${$ls(`common.at`)}`.toLowerCase()}`)}`}
                                </p>
                            </div>
                        </div>
                    {/if}
                {/await}
            </div>
        </div>
    {/if}
</EnvelopeLower>
