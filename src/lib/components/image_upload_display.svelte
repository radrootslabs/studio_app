<script lang="ts">
    import { fs } from "$lib/client";
    import {
        Glyph,
        ImageBlob,
        type CallbackPromise,
        type CallbackPromiseGeneric,
    } from "@radroots/svelte-lib";

    export let basis: {
        loading: boolean;
        file_paths: string[];
        callback_add: CallbackPromise;
        callback_edit: CallbackPromiseGeneric<number>;
    };
</script>

<div
    class={`flex flex-row h-[12rem] w-full px-4 gap-4 justify-center items-center`}
>
    <div class={`flex flex-col h-[11rem] gap-1 justify-center items-center`}>
        <button
            on:click={async () => {
                await basis.callback_edit(0);
            }}
            class={`group relative flex flex-col h-36 w-36 justify-center items-center rounded-[4.2rem] border-edge border-layer-0-glyph active:border-layer-0-glyph/60 overflow-hidden el-re`}
        >
            {#if basis.file_paths[0]}
                {#await fs.read_bin(basis.file_paths[0]) then file_data}
                    <ImageBlob
                        basis={{
                            data: file_data,
                        }}
                    />
                {/await}
            {/if}
            <div
                class={`z-10 absolute bottom-0 flex flex-row h-8 w-full justify-center items-start bg-layer-2-surface active:bg-layer-1-surface`}
            >
                <p
                    class={`font-circ font-[500] text-[0.8rem] tracking-tight text-layer-0-glyph group-active:text-layer-0-glyph/60 el-re`}
                >
                    {`Primary photo`}
                </p>
            </div>
        </button>
    </div>
    <div
        class={`grid grid-cols-12 flex flex-row gap-y-2 justify-start items-center`}
    >
        {#each Array(6).fill(0) as _, li_i}
            <div
                class={`col-span-4 flex flex-row pr-2 justify-start items-center`}
            >
                <button
                    class={`group flex flex-row h-[2.8rem] w-[2.8rem] justify-center items-center rounded-touch border-line border-layer-0-glyph/90 active:border-layer-0-glyph/60 overflow-hidden`}
                    on:click={async () => {
                        if (basis.file_paths[li_i + 1])
                            await basis.callback_edit(li_i + 1);
                        else await basis.callback_add();
                    }}
                >
                    {#if basis.file_paths[li_i + 1]}
                        {#await fs.read_bin(basis.file_paths[li_i + 1]) then file_data}
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
                                key: `image-square`,
                            }}
                        />
                    {/if}
                </button>
            </div>
        {/each}
    </div>
</div>
