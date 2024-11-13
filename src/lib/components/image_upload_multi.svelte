<script lang="ts">
    import { dialog, fs } from "$lib/client";
    import {
        app_layout,
        envelope_visible,
        EnvelopeLower,
        Glyph,
        ImageBlob,
        t,
        time_iso,
    } from "@radroots/svelte-lib";
    import {
        format_file_bytes,
        list_assign,
        list_move_index,
        parse_file_name,
    } from "@radroots/utils";
    import { fade } from "svelte/transition";

    export let photo_paths: string[];

    export let basis: {
        id: string;
    };

    let photo_file_path_0: string;
    let photo_file_path_1: string;
    let photo_file_path_2: string;
    let photo_file_path_3: string;
    let photo_file_path_4: string;
    let photo_file_path_5: string;
    let photo_file_path_6: string;
    let photo_edit_envelope: { index: number; file_path: string } | undefined;

    $: envelope_visible.set(!!photo_edit_envelope);

    $: if (photo_paths.length > 0) {
        if (photo_paths[0]) photo_file_path_0 = photo_paths[0];
        if (photo_paths[1]) photo_file_path_1 = photo_paths[1];
        if (photo_paths[2]) photo_file_path_2 = photo_paths[2];
        if (photo_paths[3]) photo_file_path_3 = photo_paths[3];
        if (photo_paths[4]) photo_file_path_4 = photo_paths[4];
        if (photo_paths[5]) photo_file_path_5 = photo_paths[5];
        if (photo_paths[6]) photo_file_path_6 = photo_paths[6];
    }

    const handle_photo_add = async (): Promise<void> => {
        try {
            const photo_paths_select = await dialog.open_photos();
            if (!photo_paths_select) return;
            console.log(
                `photo_paths_select.results[0] `,
                photo_paths_select.results[0],
            );
            photo_paths = list_assign(photo_paths, photo_paths_select.results);
        } catch (e) {
            console.log(`(error) handle_photo_add `, e);
        }
    };

    const handle_photo_envelope_edit = async (
        opts_photo_index: number,
    ): Promise<void> => {
        try {
            if (!photo_paths[opts_photo_index]) {
                photo_edit_envelope = undefined;
                return;
            }
            photo_edit_envelope = {
                index: opts_photo_index,
                file_path: photo_paths[opts_photo_index],
            };
        } catch (e) {
            console.log(`(error) handle_photo_envelope_edit `, e);
        }
    };

    const handle_photo_envelope_edit_move = async (
        opts_photo_index: number,
    ): Promise<void> => {
        try {
            photo_paths = list_move_index(photo_paths, opts_photo_index, 0);
        } catch (e) {
            console.log(`(error) handle_photo_envelope_edit_move `, e);
        }
    };
</script>

<div class={`flex flex-col w-full px-4 justify-start items-center`}>
    <button
        id={basis.id}
        class={`flex flex-row h-[11rem] w-[22rem] justify-center items-center bg-layer-1-surface rounded-[2rem] overflow-hidden`}
    >
        <button
            on:click|stopPropagation={async () => {
                if (photo_file_path_0) await handle_photo_envelope_edit(0);
                else await handle_photo_add();
            }}
            class={`group relative flex flex-col h-[11rem] w-[11rem] justify-center items-center border-r-line border-layer-0-glyph_d rounded-tl-3xl rounded-bl-3xl overflow-hidden el-re`}
        >
            {#if photo_file_path_0}
                {#await fs.read_bin(photo_file_path_0) then file_data}
                    <ImageBlob
                        basis={{
                            data: file_data,
                        }}
                    />
                {/await}
            {:else}
                <div
                    in:fade={{ duration: 200 }}
                    out:fade={{
                        delay: 0,
                        duration: 50,
                    }}
                    class={`flex flex-row justify-start items-center`}
                >
                    <Glyph
                        basis={{
                            classes: `text-layer-0-glyph group-active:text-layer-0-glyph/60 delay-100 duration-300 ease-in-out transition-all`,
                            dim: `lg`,
                            key: `camera`,
                        }}
                    />
                </div>
            {/if}
        </button>
        <div
            class={`flex flex-row flex-wrap h-full w-[11rem] justify-start items-start`}
        >
            <button
                class={`flex flex-row h-[5.5rem] w-[calc(11rem/3)] justify-center items-center border-b-line border-r-line border-layer-0-glyph_d overflow-hidden`}
                on:click|stopPropagation={async () => {
                    if (photo_file_path_1) await handle_photo_envelope_edit(1);
                    else await handle_photo_add();
                }}
            >
                {#if photo_file_path_1}
                    {#await fs.read_bin(photo_file_path_1) then file_data}
                        <ImageBlob
                            basis={{
                                data: file_data,
                            }}
                        />
                    {/await}
                {:else}
                    <Glyph
                        basis={{
                            classes: `text-layer-0-glyph text-layer-0-glyph/80 group-active:text-layer-0-glyph/60 el-re`,
                            dim: `sm`,
                            key: `plus`,
                        }}
                    />
                {/if}
            </button>
            <button
                class={`flex flex-row h-[5.5rem] w-[calc(11rem/3)] justify-center items-center border-b-line border-r-line border-layer-0-glyph_d overflow-hidden`}
                on:click|stopPropagation={async () => {
                    if (photo_file_path_2) await handle_photo_envelope_edit(2);
                    else await handle_photo_add();
                }}
            >
                {#if photo_file_path_2}
                    {#await fs.read_bin(photo_file_path_2) then file_data}
                        <ImageBlob
                            basis={{
                                data: file_data,
                            }}
                        />
                    {/await}
                {:else}
                    <Glyph
                        basis={{
                            classes: `text-layer-0-glyph text-layer-0-glyph/80 group-active:text-layer-0-glyph/60 el-re`,
                            dim: `sm`,
                            key: `plus`,
                        }}
                    />
                {/if}
            </button>
            <button
                class={`flex flex-row h-[5.5rem] w-[calc(11rem/3)] justify-center items-center border-b-line border-layer-0-glyph_d overflow-hidden`}
                on:click|stopPropagation={async () => {
                    if (photo_file_path_3) await handle_photo_envelope_edit(3);
                    else await handle_photo_add();
                }}
            >
                {#if photo_file_path_3}
                    {#await fs.read_bin(photo_file_path_3) then file_data}
                        <ImageBlob
                            basis={{
                                data: file_data,
                            }}
                        />
                    {/await}
                {:else}
                    <Glyph
                        basis={{
                            classes: `text-layer-0-glyph text-layer-0-glyph/80 group-active:text-layer-0-glyph/60 el-re`,
                            dim: `sm`,
                            key: `plus`,
                        }}
                    />
                {/if}
            </button>
            <button
                class={`flex flex-row h-[5.5rem] w-[calc(11rem/3)] justify-center items-center border-r-line border-layer-0-glyph_d overflow-hidden`}
                on:click|stopPropagation={async () => {
                    if (photo_file_path_4) await handle_photo_envelope_edit(4);
                    else await handle_photo_add();
                }}
            >
                {#if photo_file_path_4}
                    {#await fs.read_bin(photo_file_path_4) then file_data}
                        <ImageBlob
                            basis={{
                                data: file_data,
                            }}
                        />
                    {/await}
                {:else}
                    <Glyph
                        basis={{
                            classes: `text-layer-0-glyph text-layer-0-glyph/80 group-active:text-layer-0-glyph/60 el-re`,
                            dim: `sm`,
                            key: `plus`,
                        }}
                    />
                {/if}
            </button>
            <button
                class={`flex flex-row h-[5.5rem] w-[calc(11rem/3)] justify-center items-center border-r-line border-layer-0-glyph_d overflow-hidden`}
                on:click|stopPropagation={async () => {
                    if (photo_file_path_5) await handle_photo_envelope_edit(5);
                    else await handle_photo_add();
                }}
            >
                {#if photo_file_path_5}
                    {#await fs.read_bin(photo_file_path_5) then file_data}
                        <ImageBlob
                            basis={{
                                data: file_data,
                            }}
                        />
                    {/await}
                {:else}
                    <Glyph
                        basis={{
                            classes: `text-layer-0-glyph text-layer-0-glyph/80 group-active:text-layer-0-glyph/60 el-re`,
                            dim: `sm`,
                            key: `plus`,
                        }}
                    />
                {/if}
            </button>
            <button
                class={`flex flex-row h-[5.5rem] w-[calc(11rem/3)] justify-center items-center border-layer-0-glyph_d overflow-hidden`}
                on:click|stopPropagation={async () => {
                    if (photo_file_path_6) await handle_photo_envelope_edit(6);
                    else await handle_photo_add();
                }}
            >
                {#if photo_file_path_6}
                    {#await fs.read_bin(photo_file_path_6) then file_data}
                        <ImageBlob
                            basis={{
                                data: file_data,
                            }}
                        />
                    {/await}
                {:else}
                    <Glyph
                        basis={{
                            classes: `text-layer-0-glyph text-layer-0-glyph/80 group-active:text-layer-0-glyph/60 el-re`,
                            dim: `sm`,
                            key: `plus`,
                        }}
                    />
                {/if}
            </button>
        </div>
    </button>
    <div
        class={`flex flex-row h-8 w-${$app_layout} justify-start items-center`}
    >
        <div class={`flex flex-row w-[11rem] justify-center items-center`}>
            <p
                class={`font-sans font-[500] text-[0.9rem] text-layer-0-glyph/80 scale-y-[94%]`}
            >
                {`${$t(`icu.primary_*`, { value: `${$t(`common.photo`)}` })}`}
            </p>
        </div>
    </div>
</div>
<EnvelopeLower
    basis={{
        close: async () => {
            photo_edit_envelope = undefined;
        },
    }}
>
    {#if photo_edit_envelope}
        <div
            class={`flex flex-col w-full px-4 gap-4 justify-start items-center`}
        >
            <div
                class={`flex flex-row w-full justify-center items-center round-44 overflow-hidden`}
            >
                {#await fs.read_bin(photo_edit_envelope.file_path) then file_data}
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
                        if (!photo_edit_envelope) {
                            photo_edit_envelope = undefined;
                            return;
                        } else if (photo_edit_envelope.index === 0) return;
                        await handle_photo_envelope_edit_move(
                            photo_edit_envelope.index,
                        );
                        photo_edit_envelope = undefined;
                    }}
                >
                    <p class={`font-sans font-[600] text-lg text-white`}>
                        {#if photo_edit_envelope.index === 0}
                            {`${$t(`icu.primary_*`, { value: `${$t(`common.photo`)}`.toLowerCase() })}`}
                        {:else}
                            {`${$t(`common.make_primary`)}`}
                        {/if}
                    </p>
                </button>
                {#await fs.info(photo_edit_envelope.file_path) then fs_info}
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
                                    {`${$t(`common.file_name`)}:`}
                                </p>
                                <p
                                    class={`col-span-8 font-sans font-[400] text-[1.05rem] text-layer-0-glyph`}
                                >
                                    {`${parse_file_name(photo_edit_envelope.file_path)}`}
                                </p>
                            </div>
                            <div
                                class={`flex flex-col gap-1 justify-start items-start`}
                            >
                                <p
                                    class={`col-span-8 font-sans font-[400] text-[1.05rem] text-layer-0-glyph`}
                                >
                                    {`${$t(`common.file_size`)}:`}
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
                                    {`${$t(`common.date_created`)}:`}
                                </p>
                                <p
                                    class={`col-span-8 font-sans font-[400] text-[1.05rem] text-layer-0-glyph`}
                                >
                                    {`${time_iso(fs_info.birthtime?.toISOString(), `file_info`).replaceAll(`,`, ` ${`${$t(`common.at`)}`.toLowerCase()}`)}`}
                                </p>
                            </div>
                            <div
                                class={`flex flex-col gap-1 justify-start items-start`}
                            >
                                <p
                                    class={`col-span-8 font-sans font-[400] text-[1.05rem] text-layer-0-glyph`}
                                >
                                    {`${$t(`common.date_modified`)}:`}
                                </p>
                                <p
                                    class={`col-span-8 font-sans font-[400] text-[1.05rem] text-layer-0-glyph`}
                                >
                                    {`${time_iso(fs_info.mtime?.toISOString(), `file_info`).replaceAll(`,`, ` ${`${$t(`common.at`)}`.toLowerCase()}`)}`}
                                </p>
                            </div>
                        </div>
                    {/if}
                {/await}
            </div>
        </div>
    {/if}
</EnvelopeLower>
