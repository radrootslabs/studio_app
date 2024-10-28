<script lang="ts">
    import { fs } from "$lib/client";
    import {
        Glyph,
        ImageBlob,
        type CallbackPromise,
    } from "@radroots/svelte-lib";

    export let basis: {
        loading: boolean;
        list: {
            file_path: string;
        }[];
        callback_add: CallbackPromise;
    };
</script>

<div
    class={`flex flex-row h-[12rem] w-full pl-12 pr-4 gap-4 justify-start items-center`}
>
    <div class={`flex flex-col h-[11rem] gap-1 justify-center items-center`}>
        <div
            class={`relative flex flex-col h-36 w-36 justify-center items-center rounded-full border-edge border-layer-0-glyph ${basis.list.length === 0 ? `group-active:border-layer-0-glyph/60 ` : ``} overflow-hidden delay-100 duration-300 ease-in-out transition-all`}
        >
            {#if basis.list[0]?.file_path}
                {#await fs.read_bin(basis.list[0].file_path) then file_data}
                    <ImageBlob
                        basis={{
                            data: file_data,
                        }}
                    />
                {/await}
            {/if}
            <button
                class={`z-10 group absolute bottom-0 flex flex-row h-8 w-full justify-center items-start bg-layer-2-surface active:bg-layer-1-surface`}
                on:click={async () => {}}
            >
                <p
                    class={`font-circ font-[500] text-[0.8rem] tracking-tight text-layer-0-glyph group-active:text-layer-0-glyph/60 el-re`}
                >
                    {`Primary photo`}
                </p>
            </button>
        </div>
    </div>
    <div
        class={`grid grid-cols-12 flex flex-row w-full gap-y-2 justify-start items-center`}
    >
        {#each Array(6).fill(0) as li}
            <div
                class={`col-span-4 flex flex-row pr-2 justify-start items-center`}
            >
                <button
                    class={`group flex flex-row h-[2.8rem] w-[2.8rem] justify-center items-center rounded-full border-line border-layer-0-glyph/90 active:border-layer-0-glyph/60`}
                    on:click={async () => {}}
                >
                    <Glyph
                        basis={{
                            classes: `text-layer-0-glyph text-layer-0-glyph/80 group-active:text-layer-0-glyph/60 el-re`,
                            dim: `sm`,
                            key: `image-square`,
                        }}
                    />
                </button>
            </div>
        {/each}
    </div>
</div>
