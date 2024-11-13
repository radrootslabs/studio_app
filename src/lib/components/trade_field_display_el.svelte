<script lang="ts">
    import { fmt_cl, Glyph, type IClOpt } from "@radroots/svelte-lib";

    export let basis: {
        visible: boolean;
        label: string;
        display: IClOpt & {
            undef?: string;
            value: string;
            nostyle?: boolean;
            hide?: boolean;
        };
        arrow_visible?: boolean;
        slot_before?: boolean;
    };
    $: basis = basis;

    $: classes_undef =
        basis.visible && `value` in basis.display && !basis.display.value
            ? `opacity-60`
            : ``;
</script>

<div class={`flex flex-row h-6 w-full justify-between items-center`}>
    <p
        class={`font-sans font-[400] text-layer-1-glyph_d text-[1.05rem] capitalize`}
    >
        {basis.label}
    </p>
    {#if basis.visible}
        <button
            class={`flex flex-row max-w-[220px] gap-1 justify-center items-center`}
            on:click={async () => {}}
        >
            {#if basis.slot_before}
                <slot />
            {/if}
            {#if !basis.display.hide}
                <p
                    class={`${fmt_cl(basis.display.classes)} font-sans font-[400] text-[1.05rem] text-justify truncate text-layer-1-glyph_d ${classes_undef} ${basis.display.nostyle ? `` : `capitalize`}`}
                >
                    {basis.display.value || basis.display.undef || ``}
                </p>
            {/if}
            {#if !basis.slot_before}
                <slot />
            {/if}
            {#if basis.arrow_visible}
                <Glyph
                    basis={{
                        classes: `text-layer-0-glyph ${classes_undef}  pt-1`,
                        dim: `xs`,
                        key: `caret-right`,
                    }}
                />
            {/if}
        </button>
    {/if}
</div>
