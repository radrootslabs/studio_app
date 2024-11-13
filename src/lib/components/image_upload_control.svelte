<script lang="ts">
    import { dialog, fs } from "$lib/client";
    import { app_layout, Glyph, ImageBlob, t } from "@radroots/svelte-lib";
    import { list_assign } from "@radroots/utils";
    import { fade } from "svelte/transition";

    export let photo_paths: string[];
    export let photo_edit: { index: number; file_path: string } | undefined;

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
            if (!photo_paths[opts_photo_index]) photo_edit = undefined;
            else
                photo_edit = {
                    index: opts_photo_index,
                    file_path: photo_paths[opts_photo_index],
                };
        } catch (e) {
            console.log(`(error) handle_photo_envelope_edit `, e);
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
