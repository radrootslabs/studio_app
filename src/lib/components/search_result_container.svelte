<script lang="ts">
    import type { CallbackPromise } from "@radroots/svelte-lib";

    export let basis: {
        callback: CallbackPromise;
    };
</script>

<button
    class={`group flex flex-row h-[4rem] w-full px-4 justify-between items-center bg-layer-1-surface round-24 layer-1-active-surface el-re`}
    on:click={async () => {
        await basis.callback();
    }}
>
    <slot />
</button>
