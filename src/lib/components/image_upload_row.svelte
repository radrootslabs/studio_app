<script lang="ts">
    import { fs } from "$lib/client";
    import {
        Glyph,
        ImageBlob,
        Loading,
        t,
        type CallbackPromise,
    } from "@radroots/svelte-lib";
    import { fade } from "svelte/transition";

    export let basis: {
        loading: boolean;
        list: {
            file_path: string;
        }[];
        callback_add: CallbackPromise;
    };
</script>

<div
    class={`flex flex-row w-[100vw] ${basis.list.length > 1 ? `justify-start` : `justify-center`} items-center overflow-x-auto scroll-hide`}
>
    <div
        class={`flex flex-row ${basis.list.length > 1 ? `px-24` : ``} space-x-4 delay-[200ms] duration-[500ms] ease-in-out transition-all`}
    >
        <button
            in:fade={{ duration: 200 }}
            out:fade={{ delay: 0, duration: 200 }}
            on:click={async () => {
                if (basis.list.length === 0) await basis.callback_add();
            }}
            class={`group flex flex-col gap-2 justify-start items-center`}
        >
            <div
                class={`flex flex-row ${basis.list.length ? `h-52 w-52` : `h-36 w-36`} justify-center items-center rounded-full border-edge border-layer-0-glyph ${basis.list.length === 0 ? `group-active:border-layer-0-glyph/60 ` : ``} overflow-hidden delay-100 duration-300 ease-in-out transition-all`}
            >
                {#if basis.list.length}
                    {#await fs.read_bin(basis.list[0].file_path) then file_data}
                        <ImageBlob
                            basis={{
                                data: file_data,
                            }}
                        />
                    {/await}
                {:else if basis.loading}
                    <div
                        in:fade={{ duration: 200 }}
                        out:fade={{
                            delay: 0,
                            duration: 50,
                        }}
                        class={`flex flex-row justify-start items-center`}
                    >
                        <Loading />
                    </div>
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
                                classes: `text-layer-0-glyph group-active:text-layer-0-glyph/60 el-re`,
                                dim: `lg`,
                                weight: `bold`,
                                key: `download-simple`,
                            }}
                        />
                    </div>
                {/if}
            </div>
            <div class={`flex flex-row justify-start items-center`}>
                <button
                    class={`flex flex-row justify-center items-center`}
                    on:click={async () => {
                        await basis.callback_add();
                    }}
                >
                    <p
                        class={`font-sans font-[500] text-[1.1rem] tracking-tight text-layer-0-glyph group-active:text-layer-0-glyph/60 el-re`}
                    >
                        {`${$t(`icu.add_*`, { value: `${$t(`common.photo`)}`.toLowerCase() })}`}
                    </p>
                </button>
            </div>
        </button>
        {#if basis.list.length > 1}
            {#each basis.list.slice(1) as li}
                <button
                    class={`flex flex-row h-52 w-52 justify-center items-center rounded-full border-edge border-layer-0-glyph group-active:border-layer-0-glyph/60 overflow-hidden delay-100 duration-300 ease-in-out transition-all fade-in`}
                    on:click={async () => {}}
                >
                    {#await fs.read_bin(li.file_path) then file_data}
                        <ImageBlob
                            basis={{
                                data: file_data,
                            }}
                        />
                    {/await}
                </button>
            {/each}
        {/if}
    </div>
</div>
